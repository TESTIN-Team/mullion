# 给 Cursor 的任务队列

按 PROTOCOL.md 认领与回复。新任务追加在文件末尾。

## T-001 环境握手(PING)
- 背景:建立 GLM ⇄ Cursor 的双向通道,先互相确认对方环境和能力。
- 要求:在 mullion 仓库根目录执行门禁三连(fmt --check / clippy -D warnings / test),记录原始输出摘要;顺带报告你这边可用的能力(能否运行 cargo、能否读取 docs/、你的模型名)。
- 验收:`docs/coordination/outbox-from-cursor.md` 出现 R-001,含三项门禁的通过状态与测试计数;凭证落 `docs/receipts/20261010-cursor-ping.md`。
- 状态: DONE（凭证 `docs/receipts/20261010-cursor-ping.md`，回复 R-001）

## T-002 双击检测(core)
- 背景:roadmap 里的交互完整化项;协议跑通后的第一个代码任务,规模刻意收小。
- 要求:给 `Ctx::interact` 的 `Response` 增加 `double_clicked: bool`。判定:与上一次完整 click 的时间差 < 500ms 且指针位移 < 6 逻辑像素。时间来源:`FrameInput` 增加单调毫秒字段(如 `time_ms: f64`),由 host 用 `Instant` 累积填充——core 保持可测、无时钟依赖。状态存 `Memory`(上次 click 的时间与位置)。host 侧 `window.rs` 填充该字段;gallery 里"点我"按钮双击改为计数 +10(单击仍 +1)作为演示。
- 验收:新增单元测试至少覆盖——间隔 400ms 的两次 click 判定为双击;间隔 600ms 不判定;间隔短但位移 20px 不判定。三连门禁全绿;smoke 帧哈希不变(纯交互逻辑不影响静态帧)。凭证照旧落 docs/receipts/,回复 R-002。
- 状态: DONE（凭证 `docs/receipts/20261010-double-click.md`，回复 R-002）

## T-003 悬停提示 tooltip(core + gallery)
- 背景:复用 T-002 刚落地的 `time_ms` 管线做第二个交互件,roadmap 交互完整化项。
- 要求:新增 `widgets::tooltip(ctx, id, rect, text)`:指针在 `rect` 内悬停 ≥ 800ms(`FrameInput.time_ms` 判定,状态按 id 存 `Memory`,悬停中断或离开即复位)时,在新图层于指针右上方绘制提示气泡(圆角、surface 底、边框、text_dim 文本),整体夹在屏幕/窗口可视区内。gallery 给滑条行加一个 tooltip(文案如"拖动调整数值,方向键微调")。
- 验收:注入时间单测至少覆盖——悬停 800ms 出现、600ms 未出现、中途离开后重新计时;smoke 帧哈希不变(smoke 无悬停);三连门禁全绿;凭证 + R-003 照旧。
- 状态: DONE（凭证 `docs/receipts/20261010-tooltip.md`，回复 R-003）

## T-004 下拉框 ComboBox(core + gallery)
- 背景:roadmap 控件完整化项;图层/捕获/焦点机制都已就位,这是对它们的组合考验。
- 要求:`widgets::combo(ctx, rect, id, selected: usize, options: &[&str]) -> usize`。闭合态画成带 ▾ 的按钮(当前项文本);点击展开:弹层从 rect 下沿列出全部选项(悬停高亮、当前项标记),选项点击即选中并关闭;点击弹层外任意处关闭且不改变选择(提示:可在弹层下垫一个全屏交互矩形吃掉这次点击);焦点在控件上时 Up/Down 移动候选、Enter 选中、Esc 关闭。展开状态存 `Memory`(按 id)。弹层用 `FrameInput::screen` 夹取,可复用/公开 `fit_inside`。
- 验收:注入输入的单测至少覆盖——点击展开、选项点击选中并关闭、点外关闭不改选择、键盘 Up/Down+Enter 选中、Esc 关闭;三连门禁全绿;smoke 帧哈希不变(smoke 无点击);gallery 在主面板加一个下拉(如主题色强调色选择);凭证 + R-004 照旧。
- 状态: CANCELLED(2026-10-10:UI 层切换 egui,ComboBox 由 egui 内建,任务失去意义;见 D-005 之外的 D-012 与 outbox 通知)

## T-005 gallery 控件巡礼补全(egui)
- 背景:D-012 切换后 gallery 只覆盖了基础控件;把 egui 内建控件的高频项补齐,顺便当后端的兼容性巡检。
- 要求:在 gallery 增加并演示——`egui::color_picker`(`color_picker_button` 绑定强调色,替换现在的三选一下拉)、菜单栏(`egui::MenuBar`(顶层 `TopBottomPanel::top`),含"文件/主题/关于"三项,主题项切换明暗)、`egui::CollapsingHeader`(折叠"关于"区块)、`egui::DragValue`(数值行,与滑条联动)。保持中文文案与现有布局风格;不动 `mullion-host` 的后端代码。
- 验收:三连门禁全绿;`--smoke` 正常出图且 JSON 统计完整(帧哈希会因 UI 变化而改变,凭证里说明即可);新 PNG 凭证落 docs/receipts/ 并做一次像素级自查(背景/强调色/文字墨迹存在);回复 R-005 照旧。
- 状态: OPEN
