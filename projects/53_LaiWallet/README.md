# LaiBrowserWallet Browser Extension
LaiBrowserWallet 是一个基于浏览器的加密钱包扩展，旨在为用户提供安全、便捷的区块链交互体验。它实现了 EIP-1193（以太坊提供者 API）和 EIP-6963（多注入提供者发现），目前支持账户显示功能。关于dapp与钱包的连接，可以阅读[连接指南](https://learnblockchain.cn/article/7300)。
## 项目状态
    当前版本是一个早期原型，包含以下功能：
        账户显示：用户可以查看钱包中的账户信息。

        EIP-1193 支持：实现了标准的以太坊提供者 API，允许与兼容的 DApp 交互。

        EIP-6963 支持：支持多提供者发现，增强与现代 DApp 的兼容性。

    尚未实现的功能：
        转账页面：发送交易的界面和逻辑。

        DApp 连接：与去中心化应用（DApp）的完整连接流程。

    技术栈
        前端：React, TypeScript, @vanilla-extract/sprinkles（用于样式）。

        构建工具：Webpack 5, pnpm。

    区块链交互： viem、 wagmi。

    目标环境：浏览器扩展（Chrome）。

## 安装
    前提条件
    Node.js（推荐 v18.x 或更高版本）。

    pnpm（推荐 v8.x 或更高版本）。

### 步骤
    克隆仓库：
    bash

    git clone https://github.com/mosesxiaoqi/LaiBrowserWallet.git
    cd LaiBrowserWallet/browser-extension
    ## Install project dependencies

    ```bash
    pnpm install
    ```

    Copy `.env.local.example` into your own `.env.local` file in the root folder:

    ```bash
    cp .env.local
    ```

    Make sure you have these variables:

    ```bash
    EIP155_PRIVATE_KEY=
    SOLANA_PRIVATE_KEY=
    ```

    ## Importing the extension

    ### 1. Build the extension

    ```bash
    pnpm build
    ```

    ### 2. Enable Developer Mode in Chrome

    Go to `chrome://extensions/` and enable `Developer mode`.

    ### 3. Import the extension

    Click on `Load unpacked` and select the `dist` folder.

    ## Development

    ### 1. Start the development build

    ```bash
    pnpm dev
    ```

    ### 2. Make changes to the code

    Any changes to your code will trigger an update to the extension.
## 项目结构
    LaiBrowserWallet/
    |——  browser-extension/
    |    ├── dist/                    # 构建输出目录，包含打包后的扩展文件
    |    ├── node_modules/            # 依赖目录，由 pnpm 管理
    |    ├── src/                     # 源代码目录
    │    │   ├── assets/
    │    │   │   ├── images/
    │    │   │   │   ├──eth.png
    │    │   │   │   └──sol.png
    │    │   │   └── icon.png
    |    │   ├── components/          # 可复用组件
    |    │   │   ├── ArrowRightUp/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Box/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Checkmark/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Copy/
    |    │   │   │   └── index.tsx
    |    │   │   ├── IconButton/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Logo/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Switch/
    |    │   │   │   └── index.tsx
    |    │   │   ├── Text/
    |    │   │   │   └── index.tsx
    |    │   │   └── Zorb/
    |    │   │       └── index.tsx
    |    │   ├── core/                # 样式文件
    |    │   │   ├── EvmProvider.ts
    |    │   │   ├── SolanaProviderts
    |    │   │   ├── transport.ts  # Vanilla Extract 样式定义（注意拼写）
    |    │   │   └── wagmi.ts
    |    │   ├── css/                # 样式文件
    |    │   │   ├── atmos.ts
    |    │   │   ├── reset.css.ts
    |    │   │   ├── sprinkless.css.ts  # Vanilla Extract 样式定义（注意拼写）
    |    │   │   ├── touchableStyles.css.ts
    |    │   │   └── touchableStyles.ts
    |    │   ├── hooks/              # 页面组件
    |    │   │   └── useBalance.ts
    |    │   ├── pages/              # 页面组件
    |    │   │   └── Home/           # 主页组件
    |    │   │       └── index.tsx   # Home 页面实现
    |    │   ├── utils/              # 页面组件
    |    │   │   ├── AccountUtil.ts
    |    │   │   ├── ConstantsUtil.ts
    |    │   │   └── HelperUtil.ts/
    ⏐    │   ├── App.tsx             # 主应用组件，整合所有页面和组件
    ⏐    │   ├── background.ts
    ⏐    │   ├── content.ts
    ⏐    │   ├── globals.css.ts
    ⏐    │   ├── index.html
    ⏐    │   ├── index.tsx       # 入口文件，渲染 App 并注入扩展逻辑
    ⏐    │   ├── inpage.ts
    |    │   └── manifest.json
    │    ├── .babelrc
    |    ├── .env.local
    |    ├── package.json            # 项目依赖和脚本配置
    |    ├── tsconfig.json           # TypeScript 配置文件（假设存在）
    |    └── webpack.config.js       # Webpack 构建配置文件
    └──README.md
