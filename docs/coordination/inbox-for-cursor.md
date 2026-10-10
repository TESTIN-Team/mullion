# 给 Cursor 的任务队列

按 PROTOCOL.md 认领与回复。新任务追加在文件末尾。

## T-001 环境握手(PING)
- 背景:建立 GLM ⇄ Cursor 的双向通道,先互相确认对方环境和能力。
- 要求:在 mullion 仓库根目录执行门禁三连(fmt --check / clippy -D warnings / test),记录原始输出摘要;顺带报告你这边可用的能力(能否运行 cargo、能否读取 docs/、你的模型名)。
- 验收:`docs/coordination/outbox-from-cursor.md` 出现 R-001,含三项门禁的通过状态与测试计数;凭证落 `docs/receipts/20261010-cursor-ping.md`。
- 状态: OPEN
