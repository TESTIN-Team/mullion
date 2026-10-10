# AGENTS.md

mullion:**egui 的 Win32 + WGL 后端**(纯 Rust,不用 winit/eframe)。公共仓库:https://github.com/TESTIN-Team/mullion

## 任何任务开始前

1. 读 `docs/coordination/inbox-for-cursor.md`,按 `docs/coordination/PROTOCOL.md` 认领和处理任务;没有待办时不要自行发明大改动。
2. 有给 GLM 的问题或任务,写 `docs/coordination/inbox-for-glm.md`,回复写 `docs/coordination/outbox-from-cursor.md`。

## 仓库速览

- `crates/mullion-host/src/window.rs`:Win32 窗口、消息泵→`egui::RawInput`、DPI v2、IME、剪贴板、`run_egui` 主循环、`--smoke`。
- `paint.rs`:egui 细分输出→GL 1.1 绘制(`TexturesDelta`、scissors、预乘混合、回读)。
- `wgl.rs` / `input_map.rs` / `clipboard.rs` / `png.rs`:上下文、纯函数键映射、剪贴板、PNG。
- `examples/gallery.rs`:控件总览(中文 UI)。
- `docs/decisions.md`:D-001…D-012;2026-10-10 起 UI 层为 egui(自研核心已退役,见 D-012),不要重建自研控件层。
- `docs/receipts/`:验证凭证,声称完成必须附原始命令输出。

## 门禁(改动的验收标准)

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p mullion-host --example gallery --release -- --smoke   # fb_hash 应与最新凭证一致(除非有意改变渲染)
```

注意:构建钉在 MSVC target(`.cargo/config.toml`),依赖 `tools/` 下 lld-link+xwin(见 README 的 R00 前置检查);普通 `cargo` 命令即可,无需额外参数。

## 边界

通用 GUI 后端能力 only:不接入特定游戏、不做作弊类功能、UI 层统一用 egui(不引入并行的自绘控件体系)。
