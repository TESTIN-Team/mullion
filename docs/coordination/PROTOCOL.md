# GLM ⇄ Cursor 协作协议

两个 agent 通过本目录的文件对话。规则刻意保持简单:一切可追溯、一切有凭证。

## 角色

- **GLM**(ZCode 会话):主开发方。写任务、验收结果、维护决策记录。
- **Cursor**(编辑器 agent):协作方。认领任务、实现、写凭证。

## 文件

| 文件 | 谁写 | 谁读 | 内容 |
|---|---|---|---|
| `inbox-for-cursor.md` | GLM | Cursor | 给 Cursor 的任务队列 |
| `outbox-from-cursor.md` | Cursor | GLM | Cursor 的回复、结果、凭证索引 |
| `inbox-for-glm.md` | Cursor | GLM | Cursor 给 GLM 的任务/问题 |
| `PROTOCOL.md` | 双方 | 双方 | 本文件,改协议需双方在 outbox 留言确认 |

## 任务格式

```markdown
## T-001 简短标题
- 背景:为什么做
- 要求:做什么、边界在哪
- 验收:可复核的标准(命令 + 期望输出)
- 状态: OPEN / IN-PROGRESS / DONE(附凭证路径)/ REJECTED(附原因)
```

回复用 `R-XXX` 编号对应任务,必须包含:做了什么、改了哪些文件、跑了什么命令、原始输出摘要、凭证文件路径(落在 `docs/receipts/`,命名 `YYYYMMDD-主题.md`)。

## 规则

1. **凭证文化**:声称完成 = 附上命令原始输出。没有凭证的 DONE 一律退回。
2. **门禁**:任何改动过 `cargo fmt --all -- --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 才算数。
3. **范围**:mullion 是通用 GUI 框架,只做与平台无关的界面能力和 Windows 后端;不接受任何针对特定游戏的作弊类功能。
4. **不越权**:不改 `docs/decisions.md` 的既有决策;新决策由 GLM 记录。
5. **语言**:中文沟通,代码与提交信息按仓库惯例(英文提交信息可用)。
6. **认领**:把任务状态改为 IN-PROGRESS 再动手;完成后改 DONE 并写 outbox 回复。
