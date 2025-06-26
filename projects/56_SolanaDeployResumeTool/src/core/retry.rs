use anyhow::Result;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use crate::core::types::*;

/// 智能重试机制
pub struct RetryHandler {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    backoff_multiplier: f64,
    jitter_factor: f64,
}

impl RetryHandler {
    pub fn new(config: &ResumeConfig) -> Self {
        Self {
            max_retries: config.max_retries,
            base_delay: Duration::from_millis(config.retry_delay_ms),
            max_delay: Duration::from_secs(30), // 最大延迟30秒
            backoff_multiplier: 2.0,
            jitter_factor: 0.1, // 10%的随机抖动
        }
    }

    /// 执行带重试的操作
    pub async fn retry_with_backoff<F, Fut, T>(&self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut last_error = None;
        let mut delay = self.base_delay;

        for attempt in 0..=self.max_retries {
            if attempt > 0 {
                // 添加抖动避免同时重试
                let jitter = self.calculate_jitter(delay);
                let actual_delay = delay + jitter;
                
                tracing::info!(
                    "重试第 {} 次，延迟 {:.2} 秒", 
                    attempt, 
                    actual_delay.as_secs_f64()
                );
                
                sleep(actual_delay).await;
            }

            match operation().await {
                Ok(result) => {
                    if attempt > 0 {
                        tracing::info!("操作在第 {} 次重试后成功", attempt);
                    }
                    return Ok(result);
                }
                Err(e) => {
                    tracing::warn!("第 {} 次尝试失败: {}", attempt + 1, e);
                    last_error = Some(e);
                    
                    // 检查是否应该继续重试
                    if !self.should_retry(&last_error.as_ref().unwrap()) {
                        tracing::info!("错误类型不适合重试，停止重试");
                        break;
                    }
                    
                    // 计算下次延迟时间
                    delay = self.calculate_next_delay(delay);
                }
            }
        }

        Err(last_error.unwrap_or_else(|| anyhow::anyhow!("重试次数已用完")))
    }

    /// 计算抖动时间
    fn calculate_jitter(&self, base_delay: Duration) -> Duration {
        let jitter_ms = (base_delay.as_millis() as f64 * self.jitter_factor * rand::random::<f64>()) as u64;
        Duration::from_millis(jitter_ms)
    }

    /// 计算下次延迟时间（指数退避）
    fn calculate_next_delay(&self, current_delay: Duration) -> Duration {
        let next_delay_ms = (current_delay.as_millis() as f64 * self.backoff_multiplier) as u64;
        let next_delay = Duration::from_millis(next_delay_ms);
        
        if next_delay > self.max_delay {
            self.max_delay
        } else {
            next_delay
        }
    }

    /// 判断错误是否应该重试
    fn should_retry(&self, error: &anyhow::Error) -> bool {
        let error_str = error.to_string().to_lowercase();
        
        // 网络相关错误应该重试
        if error_str.contains("timeout") || 
           error_str.contains("connection") ||
           error_str.contains("network") ||
           error_str.contains("502") ||
           error_str.contains("503") ||
           error_str.contains("504") {
            return true;
        }
        
        // Solana特定的临时错误
        if error_str.contains("blockhash not found") ||
           error_str.contains("too many requests") ||
           error_str.contains("rate limit") {
            return true;
        }
        
        // 永久性错误不应该重试
        if error_str.contains("invalid") ||
           error_str.contains("insufficient funds") ||
           error_str.contains("unauthorized") ||
           error_str.contains("forbidden") {
            return false;
        }
        
        // 默认重试
        true
    }
}

/// 断路器模式实现
pub struct CircuitBreaker {
    failure_threshold: u32,
    timeout: Duration,
    current_failures: u32,
    last_failure_time: Option<Instant>,
    state: CircuitState,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,    // 正常状态
    Open,      // 断路状态
    HalfOpen,  // 半开状态
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: Duration) -> Self {
        Self {
            failure_threshold,
            timeout,
            current_failures: 0,
            last_failure_time: None,
            state: CircuitState::Closed,
        }
    }

    /// 执行带断路器保护的操作
    pub async fn call<F, Fut, T>(&mut self, operation: F) -> Result<T>
    where
        F: Fn() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        // 检查断路器状态
        match self.state {
            CircuitState::Open => {
                // 检查是否到了重试时间
                if let Some(last_failure) = self.last_failure_time {
                    if last_failure.elapsed() > self.timeout {
                        self.state = CircuitState::HalfOpen;
                        tracing::info!("断路器进入半开状态");
                    } else {
                        return Err(anyhow::anyhow!("断路器开启，拒绝执行操作"));
                    }
                }
            }
            CircuitState::HalfOpen => {
                // 半开状态下只允许一次尝试
            }
            CircuitState::Closed => {
                // 正常状态，允许执行
            }
        }

        // 执行操作
        match operation().await {
            Ok(result) => {
                self.on_success();
                Ok(result)
            }
            Err(e) => {
                self.on_failure();
                Err(e)
            }
        }
    }

    fn on_success(&mut self) {
        self.current_failures = 0;
        self.state = CircuitState::Closed;
    }

    fn on_failure(&mut self) {
        self.current_failures += 1;
        self.last_failure_time = Some(Instant::now());

        if self.current_failures >= self.failure_threshold {
            self.state = CircuitState::Open;
            tracing::warn!("断路器开启，失败次数: {}", self.current_failures);
        }
    }

    pub fn get_state(&self) -> &CircuitState {
        &self.state
    }
}

/// 网络健康检查器
pub struct HealthChecker {
    rpc_url: String,
    check_interval: Duration,
    timeout: Duration,
}

impl HealthChecker {
    pub fn new(rpc_url: String) -> Self {
        Self {
            rpc_url,
            check_interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
        }
    }

    /// 检查网络健康状态
    pub async fn check_health(&self) -> Result<NetworkHealth> {
        let start = Instant::now();

        let client = reqwest::Client::new();
        let response = tokio::time::timeout(
            self.timeout,
            client.post(&self.rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "id": 1,
                    "method": "getHealth"
                }))
                .send()
        ).await;

        match response {
            Ok(Ok(resp)) => {
                let latency = start.elapsed();
                let status_code = resp.status().as_u16();
                
                if status_code == 200 {
                    Ok(NetworkHealth {
                        is_healthy: true,
                        latency,
                        last_check: Instant::now(),
                        error_message: None,
                    })
                } else {
                    Ok(NetworkHealth {
                        is_healthy: false,
                        latency,
                        last_check: Instant::now(),
                        error_message: Some(format!("HTTP {}", status_code)),
                    })
                }
            }
            Ok(Err(e)) => Ok(NetworkHealth {
                is_healthy: false,
                latency: start.elapsed(),
                last_check: Instant::now(),
                error_message: Some(e.to_string()),
            }),
            Err(_) => Ok(NetworkHealth {
                is_healthy: false,
                latency: self.timeout,
                last_check: Instant::now(),
                error_message: Some("请求超时".to_string()),
            }),
        }
    }

    /// 持续监控网络健康状态
    pub async fn monitor_health<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(NetworkHealth),
    {
        let mut interval = tokio::time::interval(self.check_interval);
        
        loop {
            interval.tick().await;
            
            match self.check_health().await {
                Ok(health) => callback(health),
                Err(e) => {
                    tracing::error!("健康检查失败: {}", e);
                    callback(NetworkHealth {
                        is_healthy: false,
                        latency: Duration::from_secs(0),
                        last_check: Instant::now(),
                        error_message: Some(e.to_string()),
                    });
                }
            }
        }
    }
}

/// 网络健康状态
#[derive(Debug, Clone)]
pub struct NetworkHealth {
    pub is_healthy: bool,
    pub latency: Duration,
    pub last_check: Instant,
    pub error_message: Option<String>,
}

/// 自适应超时管理器
pub struct AdaptiveTimeout {
    min_timeout: Duration,
    max_timeout: Duration,
    current_timeout: Duration,
    success_count: u32,
    failure_count: u32,
    adjustment_factor: f64,
}

impl AdaptiveTimeout {
    pub fn new() -> Self {
        Self {
            min_timeout: Duration::from_secs(5),
            max_timeout: Duration::from_secs(60),
            current_timeout: Duration::from_secs(15),
            success_count: 0,
            failure_count: 0,
            adjustment_factor: 0.1,
        }
    }

    /// 记录成功操作
    pub fn record_success(&mut self, duration: Duration) {
        self.success_count += 1;
        
        // 如果操作时间很短，可以适当降低超时时间
        if duration < self.current_timeout / 2 && self.success_count % 5 == 0 {
            let new_timeout = self.current_timeout - Duration::from_millis(
                (self.current_timeout.as_millis() as f64 * self.adjustment_factor) as u64
            );
            self.current_timeout = new_timeout.max(self.min_timeout);
        }
    }

    /// 记录失败操作
    pub fn record_failure(&mut self) {
        self.failure_count += 1;
        
        // 连续失败时增加超时时间
        if self.failure_count % 3 == 0 {
            let new_timeout = self.current_timeout + Duration::from_millis(
                (self.current_timeout.as_millis() as f64 * self.adjustment_factor * 2.0) as u64
            );
            self.current_timeout = new_timeout.min(self.max_timeout);
        }
    }

    /// 获取当前推荐的超时时间
    pub fn get_timeout(&self) -> Duration {
        self.current_timeout
    }

    /// 重置统计
    pub fn reset(&mut self) {
        self.success_count = 0;
        self.failure_count = 0;
        self.current_timeout = Duration::from_secs(15);
    }
} 