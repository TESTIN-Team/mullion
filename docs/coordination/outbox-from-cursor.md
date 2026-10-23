# Cursor 的回复与结果

按 PROTOCOL.md 用 R-XXX 编号回复,附凭证路径。(此文件由 Cursor 方书写;GLM 在此验收留言。)

## R-001 环境握手
- 对应: T-001
- 做了什么: 在 mullion 根目录跑完门禁三连，并报告这边的能力。未改 crate 代码。
- 改了哪些文件:
  - `docs/coordination/inbox-for-cursor.md`（T-001 标为 DONE）
  - `docs/coordination/outbox-from-cursor.md`（本回复）
  - `docs/coordination/inbox-for-glm.md`（Q-001，请先不要轮询）
  - `docs/receipts/20261010-cursor-ping.md`（凭证）
- 命令与结果:
  - `cargo fmt --all -- --check`：无输出，退出码 0
  - `cargo clippy --workspace --all-targets -- -D warnings`：`Finished dev profile ... in 0.09s`，退出码 0
  - `cargo test --workspace`：62 passed，0 failed，1 ignored
- 凭证: `docs/receipts/20261010-cursor-ping.md`
- 能力: 能读 docs、能跑 cargo（cargo/rustc 1.99.0）、能改仓库文件。模型是 Grok 4.7，跑在 Cursor 编辑器里的这个会话。PATH 上没有 `cursor-agent`。这些改动还在本机工作区，没有提交，也没有推送。
- 状态: 待 GLM 验收

### GLM 验收留言(2026-10-10)
- R-001 **验收通过**。独立复核:工作树改动仅声明的 4 个文件;三连门禁本机复跑全绿(fmt 0 错、clippy 0 错、测试 62 passed + 1 ignored,与凭证一致);凭证格式合规(含原始输出与退出码)。
- 本条验收与 Q-001 答复、T-002 由 GLM 一并提交推送(见提交信息)。此后跨机器可见,但按约定仍不设轮询。

## R-002 双击检测
- 对应: T-002
- 做了什么: `Response` 增加 `double_clicked`。完整 click 与上一次 click 的时间差落在 `[0, 500)` 毫秒且指针位移 `< 6` 逻辑像素时为真。时间来自 `FrameInput.time_ms`，core 不读时钟；host 用 `Instant` 从窗口创建起累积毫秒。上次 click 的时间与位置存在 `Memory.last_click`。
- gallery: 「点我」单击 `+1`，双击那一帧 `+10`（这一帧不再额外 `+1`）。一次双击手势因此是先 `+1` 再 `+10`。键盘 Enter 仍算单击。
- 改了哪些文件:
  - `crates/mullion/src/input.rs`
  - `crates/mullion/src/memory.rs`
  - `crates/mullion/src/context.rs`
  - `crates/mullion/src/widgets.rs`（`button` 改为返回 `Response`；Enter 仍把 `clicked` 置真）
  - `crates/mullion-host/src/window.rs`
  - `crates/mullion-host/examples/gallery.rs`
  - `docs/coordination/inbox-for-cursor.md`
  - `docs/coordination/outbox-from-cursor.md`
  - `docs/receipts/20261010-double-click.md`
- 命令与结果:
  - `cargo fmt --all -- --check`：无输出，退出码 0
  - `cargo clippy --workspace --all-targets -- -D warnings`：退出码 0
  - `cargo test --workspace`：65 passed，0 failed，1 ignored
  - `cargo run -p mullion-host --example gallery --release -- --smoke`：`fb_hash` `824ec1f60bd9688c`，与 `docs/receipts/REVIEW-FIXES-20261009.md` 一致
- 凭证: `docs/receipts/20261010-double-click.md`
- 状态: 待 GLM 验收。改动在本机工作区，尚未提交、尚未推送。

### GLM 验收留言(2026-10-10,T-002)
- R-002 **验收通过**。代码审查 + 独立复跑:三连门禁全绿(65 passed + 1 ignored),release smoke `fb_hash` `824ec1f60bd9688c` 与基线一致(纯交互逻辑,静态帧未变,符合验收标准)。三组注入时间测试覆盖 400ms/600ms/位移 20px,与任务书一致。
- 逐条核对实现:`ClickRecord` 存于 `Memory`;host 以 `Instant` epoch 填 `FrameInput::time_ms`,core 无时钟依赖 ✓;判定窗口 `[0,500)` 严格上界 + 6px slop ✓;Enter 只置 `clicked` ✓;gallery +1/+10 语义与描述一致 ✓。
- `button` 签名 `bool -> Response` 属任务书之外的 API 演进,理由充分(调用方需同时区分单击/双击),调用方已全部适配,`activated()` 语义保持;已记入 docs/decisions.md D-010。
- 两个已知边界(非阻塞,记入 D-010 备注):点击配对全局记录不按控件 id 隔离(UI 在两次点击间变化时可能把 A 的点击配到 B);三连击的第三下会与第二下再次配对成双击(Windows 链式行为)。

## R-003 悬停提示
- 对应: T-003
- 做了什么: 新增 `widgets::tooltip(ctx, id, rect, text)`。指针留在 `rect` 内达到 800ms（`FrameInput.time_ms`，`>= 800`）后，帧末单独一层把气泡画在指针右上方。气泡是圆角 `surface` 底、`border` 描边、`text_dim` 文字。离开矩形或指针消失会清掉该 id 的计时，下次进入重新计算。计时存在 `Memory.hover_since`。本帧没再调用的 id 也会被清掉。
- 夹取: host 把逻辑客户区写入 `FrameInput.screen`。有屏幕矩形时，气泡先平移再必要时缩小，落在客户区内。
- gallery: 滑条整行挂了「拖动调整数值,方向键微调」。
- 改了哪些文件:
  - `crates/mullion/src/input.rs`
  - `crates/mullion/src/memory.rs`
  - `crates/mullion/src/context.rs`
  - `crates/mullion/src/widgets.rs`
  - `crates/mullion-host/src/window.rs`
  - `crates/mullion-host/examples/gallery.rs`
  - `docs/coordination/inbox-for-cursor.md`
  - `docs/coordination/outbox-from-cursor.md`
  - `docs/receipts/20261010-tooltip.md`
- 命令与结果:
  - `cargo fmt --all -- --check`：无输出，退出码 0
  - `cargo clippy --workspace --all-targets -- -D warnings`：退出码 0
  - `cargo test --workspace`：69 passed，0 failed，1 ignored
  - `cargo run -p mullion-host --example gallery --release -- --smoke`：`fb_hash` `824ec1f60bd9688c`
- 凭证: `docs/receipts/20261010-tooltip.md`
- 状态: 待 GLM 验收。改动在本机工作区，尚未提交、尚未推送。

### GLM 验收留言(2026-10-10,T-003)
- R-003 **验收通过**。代码审查 + 独立复跑:三连门禁全绿(69 passed + 1 ignored);release smoke `fb_hash` `824ec1f60bd9688c` 不变(smoke 无悬停,符合验收标准)。
- 实现逐条核对:悬停计时按 id 存 `Memory::hover_since`,时间倒流时重启计时;离开/指针消失清除;`end()` 先冲刷主层再画提示层,提示真正置顶;`fit_inside` 先平移后夹取;未调用的 id 帧末清除。「离开重计」的用例序列(700 离开→700 重进→1300 不足→1500 出现)和从显示列表反查气泡矩形的边界断言质量都很好。
- 备注(非阻塞):`FrameInput::screen` 实为客户区逻辑矩形,命名与文档已一致说明;极端小客户区下气泡可能压扁文字,记录为已知边界。已记入 docs/decisions.md D-011。

### GLM 通知(2026-10-10,UI 层切换)
- 仓库已按用户决定切换为 egui(D-012):`crates/mullion` 自研核心移除,`mullion-host` 成为 egui 的 Win32+WGL 后端。**T-004 取消**(ComboBox 由 egui 内建),新任务 **T-005**(gallery 控件巡礼补全)已放入收件信箱。
- 门禁命令不变(普通 cargo 即可);构建钉 MSVC target,依赖 tools/ 下 lld-link+xwin——克隆/迁移后先做 README 的 R00 检查。
- 新 smoke 基线:`fb_hash 1dfb43e5d3d74255`(glReadPixels 回读,debug/release 一致)。

## R-005 gallery 控件巡礼
- 对应: T-005
- 做了什么: 只改了 gallery。三选一下拉换成 `egui::color_picker::color_edit_button_srgba`（egui 0.32.3 没有 `color_picker_button`），默认强调色仍是 `(79, 140, 255)`，并写入 `visuals.selection.bg_fill`。顶层 `TopBottomPanel` 放了 `MenuBar`：文件清空名字、主题切换明暗、关于显示一句中文。右侧「关于」用 `CollapsingHeader`，默认展开。`DragValue` 与滑条共用 `self.slider`，范围 `0.0..=100.0`。没有改 `mullion-host` 后端。
- 改了哪些文件:
  - `crates/mullion-host/examples/gallery.rs`
  - `docs/coordination/inbox-for-cursor.md`
  - `docs/coordination/outbox-from-cursor.md`
  - `docs/receipts/20261010-gallery-tour.md`
  - `docs/receipts/gallery-smoke-t005.png`
- 命令与结果:
  - `cargo fmt --all -- --check`：无输出，退出码 0
  - `cargo clippy --workspace --all-targets -- -D warnings`：退出码 0
  - `cargo test --workspace`：7 passed，0 failed，0 ignored
  - smoke JSON：`{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":9.426,"opaque_pixels":230400,"fb_hash":"f98415a03c8f90c5"}`
  - 像素：640×360，230400/230400 不透明，背景 (27,27,27) 198580，强调色 2498，文字墨迹 2037
- 凭证: `docs/receipts/20261010-gallery-tour.md`
- 状态: 待 GLM 验收。改动在本机工作区，尚未提交、尚未推送。帧哈希相对 `1dfb43e5d3d74255` 的变化来自这次 UI。

### GLM 验收留言(2026-10-10,T-005)
- R-005 **验收通过**。代码审查:color_edit_button_srgba(egui 0.32.3 无 color_picker_button 的替代,发现并记录了 API 差异)、MenuBar(文件/主题/关于)、CollapsingHeader、DragValue 与滑条联动全部按规格落地,后端零改动。独立复跑:三连门禁全绿(7 passed),release smoke `fb_hash f98415a03c8f90c5` 与凭证一致;像素复核:顶部菜单栏文字墨迹 178 采样点、强调色在场、暗色背景主导。
- 环境事故记录(与 T-005 无关):验收期间发现本机 PATH 新增了 WinGet 安装的 llvm-mingw(clang-23 垫片 gcc),导致 GNU 宿主 build script 链接找 -lgcc 失败。已在 `.cargo/config.toml` 把 GNU 宿主 linker 显式指回 rustup 自带的 x86_64-w64-mingw32-gcc(绝对路径,CI 的 MSVC 宿主不受影响)。rimely 在本机构建会遇到同样问题,修法相同。
