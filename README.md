# mullion

窗棂——窗户的横竖框。**egui 的 Win32 + WGL 后端**:纯 Rust、不依赖 winit/eframe,自己拥有窗口、消息泵、DPI、IME 与呈现,UI 层由 [egui](https://github.com/emilk/egui) 提供。

```
┌────────────────────────── mullion ──────────────────────────┐
│  crates/mullion-host                                       │
│    window.rs    Win32 窗口、消息泵 → egui::RawInput 翻译、   │
│                 按显示器 DPI v2、IME(组合/提交/候选跟随)、   │
│                 剪贴板、光标图标、--smoke 无头验证            │
│    paint.rs     egui 细分输出 → GL 1.1:TexturesDelta 纹理   │
│                 管理(含字形图集)、逐裁剪矩形 scissors 绘制、 │
│                 预乘混合、回读帧缓冲出 PNG 凭证               │
│    wgl.rs       WGL 兼容上下文创建/vsync/释放                │
│    input_map.rs VK → egui::Key、光标图标 → IDC_*(纯函数可测) │
│    clipboard.rs CF_UNICODETEXT 剪贴板                        │
│    png.rs       零依赖 PNG 写出(stored zlib)                 │
│  examples/gallery.rs  控件总览(中文 UI、IME、明暗主题)       │
└─────────────────────────────────────────────────────────────┘
```

历史:本仓库最初是零依赖的自研即时模式 GUI 核心(几何/排版/控件/CPU 光栅化),2026-10-10 起 UI 层切换为 egui,自研核心退役(git 历史与既有凭证完整保留,见 `docs/decisions.md` D-012)。

## 快速开始

本机无 VS/MSVC 链接器:`.cargo/config.toml` 把产物钉在 `x86_64-pc-windows-msvc`,链接用 `tools/` 下的 lld-link + xwin SDK(目录 junction 指向 rimely 的工具,已 gitignore)。另:egui 依赖链的 raw-dylib 在 GNU target 上需要 dlltool+as,本机不可用,这也是选 MSVC target 的原因之一。克隆后前置检查(R00):

```text
ls tools/bin/lld-link.exe tools/xwin-splat/crt/lib/x86_64 \
   tools/xwin-splat/sdk/lib/um/x86_64 tools/xwin-splat/sdk/lib/ucrt/x86_64
```

四者任一缺失则构建必败,先恢复工具链。

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p mullion-host --example gallery --release
```

## 无头验证(--smoke)

```text
cargo run -p mullion-host --example gallery --release -- --smoke docs/receipts/gallery-smoke-egui.png
```

隐藏窗口渲染 12 帧,第 6 帧缩放 1000×640 → 640×360,末帧 `glReadPixels` 回读后台缓冲为 PNG 凭证并打印 JSON 统计。基线:`fb_hash 1dfb43e5d3d74255`(debug/release 一致;GL 光栅化同机确定性)。凭证在 `docs/receipts/`。

## 多 agent 协作

本仓库由 GLM(ZCode)与 Cursor 通过 `docs/coordination/` 的文件信箱协作开发,任何 agent 开始工作前先读 `AGENTS.md`。技术决策见 `docs/decisions.md`(D-001…D-012)。

## 许可

MIT OR Apache-2.0,见 `LICENSE-MIT` / `LICENSE-APACHE`。
