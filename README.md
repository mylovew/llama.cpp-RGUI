# llama.cpp RGUI

> llama.cpp 启动客户端 GUI —— 用桌面图形界面可视化地启动和管理 [llama.cpp](https://github.com/ggml-org/llama.cpp) 的 `llama-server` 服务。

![screenshot](docs/screenshot.png)

---

## 功能特性

- **一键启动本地大模型服务**：选择 GGUF 模型与启动预设，一键拉起 `llama-server`，无需记忆命令行参数。
- **模型库管理**：扫描多个文件夹，自动解析 `.gguf` 文件元数据（架构、参数量、量化类型、上下文长度等）。
- **预设管理**：创建 / 复制 / 编辑 / 删除多套启动参数，覆盖 `llama-server` 全部常用参数（上下文长度、线程、GPU 层、批大小、KV 缓存等）。
- **实时状态监控**：LED 状态指示灯、PID、监听地址端口展示、崩溃自动检测。
- **实时日志查看**：右侧抽屉式日志面板，自动识别日志级别并着色。
- **一键打开聊天**：在系统默认浏览器中打开 `llama-server` 自带的 Web UI。
- **版本管理**：检测本地版本，对比 GitHub 最新 Release 提示更新。
- **显存估算**：根据模型元数据与预设参数估算 GPU / 内存占用。
- **主题切换**：跟随系统 / 浅色 / 深色三档切换，玻璃态视觉设计。
- **配置持久化**：原子化写入，记忆上次使用的模型、预设与设置。

## 技术栈

| 层级 | 技术 | 版本 |
|------|------|------|
| 桌面框架 | Tauri | 2 |
| 前端框架 | Vue | ^3.5.13 |
| 路由 | Vue Router | ^4.5.0 |
| 状态管理 | Pinia | ^2.2.6 |
| UI 组件库 | Naive UI | ^2.40.1 |
| 图标库 | @vicons/ionicons5 | ^0.13.0 |
| 工具库 | @vueuse/core | ^11.3.0 |
| 构建工具 | Vite | ^5.4.11 |
| 类型检查 | TypeScript + vue-tsc | ~5.6.3 / ^2.1.10 |
| 后端语言 | Rust | >= 1.77 |
| 异步运行时 | tokio | 1 (full) |
| HTTP 客户端 | reqwest (rustls-tls) | 0.12 |
| 并行计算 | rayon | 1 |

## 快速开始

### 方式一：从 Release 下载（推荐普通用户）

1. 前往 [Releases 页面](https://github.com/mylovew/llama.cpp-RGUI/releases) 下载对应平台的安装包：
   - **Windows**：`.msi` 或 `.exe` 安装包
   - **macOS**：`.dmg` 安装包
   - **Linux**：`.deb` / `.rpm` / `.AppImage`
2. 安装并打开应用。

> **macOS 用户注意**：本应用未进行代码签名。首次打开时 macOS 会提示"无法打开，因为来自身份不明的开发者"。
> 请**右键点击应用图标 → 选择"打开"**，在弹出的对话框中再次点击"打开"即可绕过 Gatekeeper 限制。此后可正常双击打开。

### 方式二：从源码构建

#### 环境要求

- [Node.js](https://nodejs.org/) >= 18
- [pnpm](https://pnpm.io/) >= 9（或通过 `corepack enable` 自动启用）
- [Rust](https://www.rust-lang.org/) >= 1.77
- 平台特定依赖：
  - **Windows**：WebView2（Windows 10/11 通常已预装）
  - **macOS**：Xcode Command Line Tools（`xcode-select --install`）
  - **Linux**：`libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev`

#### 构建步骤

```bash
# 1. 克隆仓库
git clone https://github.com/mylovew/llama.cpp-RGUI.git
cd llama.cpp-RGUI

# 2. 启用 pnpm（若未安装）
corepack enable

# 3. 安装依赖
pnpm install

# 4. 开发模式运行
pnpm tauri:dev

# 5. 生产构建（产物在 src-tauri/target/release/bundle/）
pnpm tauri:build
```

## 配置说明

应用首次启动会自动创建配置目录：

- **Windows**：`C:\Users\<用户>\AppData\Roaming\com.llamacpp.rgui\`
- **macOS**：`~/Library/Application Support/com.llamacpp.rgui/`
- **Linux**：`~/.config/com.llamacpp.rgui/`

配置文件采用 JSON 格式原子化写入，包含模型文件夹路径、预设列表、主题偏好等。

> 本应用**仅启动和管理** `llama-server`，你需要自行下载 `llama.cpp` 编译产物或预编译的 `llama-server` 可执行文件，并在设置中指定其路径。

## 项目结构

```
llama.cpp-RGUI/
├── src/                        # 前端源码 (Vue 3)
│   ├── App.vue                 # 根组件
│   ├── main.ts                 # 入口
│   ├── views/                  # 页面
│   │   ├── LaunchView.vue      # 启动页（模型选择 + 预设 + 启动控制）
│   │   └── SettingsView.vue    # 设置页（模型库 + 预设管理 + 通用设置）
│   ├── components/             # 组件
│   │   ├── ThemeToggle.vue     # 主题切换
│   │   └── WindowControls.vue  # 自定义窗口标题栏按钮
│   ├── stores/                 # Pinia 状态管理
│   │   └── settings.ts         # 设置 store
│   ├── router/                 # 路由配置
│   └── assets/                 # 静态资源
├── src-tauri/                  # 后端源码 (Rust)
│   ├── src/
│   │   ├── main.rs             # 入口
│   │   ├── lib.rs              # 库入口（注册命令）
│   │   ├── commands/           # Tauri 命令（前端调用的后端接口）
│   │   │   ├── server.rs       # 服务启动 / 停止 / 状态
│   │   │   ├── settings.rs     # 设置读写
│   │   │   ├── models.rs       # 模型扫描
│   │   │   └── webview.rs      # Webview 相关
│   │   ├── config/             # 配置持久化
│   │   ├── gguf/               # GGUF 二进制解析器（自实现）
│   │   │   ├── parser.rs       # 解析器
│   │   │   ├── meta.rs         # 元数据
│   │   │   ├── quant.rs        # 量化类型
│   │   │   └── vram.rs         # 显存估算
│   │   └── process/            # 跨平台进程管理
│   │       ├── spawn.rs        # 进程启动
│   │       ├── shutdown.rs     # 进程终止（Windows taskkill / Unix SIGTERM）
│   │       ├── health.rs       # 两阶段就绪检测（TCP + HTTP）
│   │       ├── log_pipe.rs     # 日志管道
│   │       └── manager.rs      # 进程管理器
│   ├── capabilities/           # Tauri 2 权限配置
│   ├── icons/                  # 应用图标
│   └── tauri.conf.json         # Tauri 配置
├── package.json
├── vite.config.ts
└── pnpm-lock.yaml
```

## 开发指南

```bash
# 安装依赖
pnpm install

# 启动开发服务器（热更新）
pnpm tauri:dev

# 类型检查
pnpm build

# 构建生产版本
pnpm tauri:build
```

### 代码规范

- 前端使用 TypeScript，提交前确保 `vue-tsc --noEmit` 无报错。
- Rust 代码提交前建议运行 `cargo fmt` 与 `cargo clippy`。

## 贡献指南

欢迎提交 Issue 和 Pull Request！

1. Fork 本仓库
2. 创建特性分支：`git checkout -b feature/your-feature`
3. 提交更改：`git commit -m "feat: add your feature"`
4. 推送分支：`git push origin feature/your-feature`
5. 提交 Pull Request

请确保提交前代码能通过类型检查与编译。

## 开源协议

本项目基于 [MIT License](./LICENSE) 开源。
