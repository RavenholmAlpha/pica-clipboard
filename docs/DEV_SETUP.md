# 开发环境配置指南 (Development Setup Guide)

本文档旨在帮助开发者在 Windows 环境下搭建 PICAclip 的开发环境。

## 1. 前置要求 (Prerequisites)

### 1.1 操作系统
- **Windows 10 或 Windows 11** (推荐 64-bit)

### 1.2 Rust 工具链
PICAclip 使用 Rust 语言开发。
1. 下载并安装 [Rustup](https://rustup.rs/) (`rustup-init.exe`)。
2. 安装过程中选择默认配置（Stable channel）。
3. 验证安装：
   ```powershell
   rustc --version
   cargo --version
   ```

### 1.3 C++ 生成工具 (Visual Studio Build Tools)
Rust 在 Windows 上需要 MSVC 链接器。Slint 编译也依赖它。
1. 下载 [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)。
2. 在安装程序中，勾选 **"Desktop development with C++" (使用 C++ 的桌面开发)** 工作负载。
3. 确保包含 **Windows SDK**。

### 1.4 Slint 依赖
Slint 在 Windows 上通常无需额外系统库（使用 Skia 或原生渲染），但建议安装 **VS Code 插件**以获得语法高亮和预览功能。
- **VS Code Extension**: `Slint` (ID: `Slint.slint`)

### 1.5 SQLite 工具 (可选)
为了调试数据库，建议安装：
- [DB Browser for SQLite](https://sqlitebrowser.org/)

## 2. 项目构建 (Build & Run)

### 2.1 克隆代码
```powershell
git clone https://github.com/your-repo/picaclip.git
cd picaclip
```

### 2.2 开发模式运行
```powershell
# 首次编译可能较慢，因为需要编译 Slint 和所有依赖
cargo run
```
*注：如果遇到链接错误，请检查是否正确安装了 C++ Build Tools。*

### 2.3 构建发布版 (Release)
```powershell
cargo build --release
```
生成的二进制文件位于 `target/release/picaclip.exe`。建议使用该版本进行性能测试。

## 3. 环境变量 (Environment Variables)

| 变量名 | 描述 | 默认值 |
| :--- | :--- | :--- |
| `RUST_LOG` | 日志级别 (e.g., `info`, `debug`, `picaclip=trace`) | `error` |
| `PICACLIP_DB_PATH` | 自定义数据库路径 | `%APPDATA%\picaclip\data.db` |
| `SLINT_DEBUG_PERFORMANCE` | 开启 Slint 性能调试覆盖层 | `not set` |

## 4. 常见问题 (Troubleshooting)

### Q1: `link.exe` not found?
**A**: 确保你安装了 Visual Studio Build Tools，并且在 PATH 中。或者重新运行 `rustup update` 确保 Rust 找到了 MSVC 工具链。

### Q2: Slint 预览不显示？
**A**: 确保 `.slint` 文件中有一个导出的组件（`export component AppWindow inherits Window { ... }`）。点击 VS Code 编辑器右上角的 "Show Preview" 按钮。

### Q3: 数据库错误 "no such table"?
**A**: 项目首次启动时会自动运行 Migration 建表。如果开发过程中修改了 Schema，建议删除旧的 `.db` 文件让其重新生成，或使用 `sqlx-cli` (如果项目集成了的话) 进行迁移。

## 5. 代码风格与规范
- **Rust**: 提交前请运行 `cargo fmt`。
- **Clippy**: 建议运行 `cargo clippy` 检查潜在问题。
