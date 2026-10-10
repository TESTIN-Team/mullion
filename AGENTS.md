# AGENTS.md

mullion:纯 Rust 即时模式 GUI 框架(核心零依赖 + Windows 后端)。公共仓库:https://github.com/TESTIN-Team/mullion

## 任何任务开始前

1. 读 `docs/coordination/inbox-for-cursor.md`,按 `docs/coordination/PROTOCOL.md` 的格式认领和处理任务;没有待办任务时不要自行发明大改动。
2. 有给 GLM 的问题或任务,写进 `docs/coordination/inbox-for-glm.md`,回复写 `docs/coordination/outbox-from-cursor.md`。

## 仓库速览

- `crates/mullion`:UI 核心(geometry/style/text/input/memory/context/layout/widgets/render),全部可无窗口单测,零依赖。
- `crates/mullion-host`:Windows 后端(Win32 窗口、WGL 呈现、GDI 字形、IME、剪贴板、PNG、--smoke)。
- `docs/decisions.md`:既有技术决策(D-001…D-009),不要推翻;新决策提给 GLM 记录。
- `docs/receipts/`:验证凭证,声称完成必须附原始命令输出。

## 门禁(改动的验收标准)

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p mullion-host --example gallery --release -- --smoke   # 帧哈希应与上次凭证一致(除非有意改变渲染)
```

## 边界

通用 GUI 能力 only:不接入特定游戏、不做作弊类功能、不引入第三方依赖到 `crates/mullion`(系统 API 声明仅限 `mullion-host`)。
