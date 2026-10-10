# 决策记录(mullion)

按时间顺序。每条含背景、决策、后果。

## D-001 纯 Rust、零依赖核心

**背景**:目标是一个可以被完整单测、可以审计的 GUI 核心;第三方 UI/字体/图像库会把大量行为拉进依赖树。
**决策**:`crates/mullion` 只用 std。字体、字形、呈现由 `FontBackend` trait 与 host crate 承担;系统 API 调用全部隔离在 `mullion-host`(windows-sys 声明 + 少量自声明 extern,这是文档化的系统 API 例外)。
**后果**:核心每个模块(含编辑框状态机、交互解析、光栅化)都有无窗口单测;换平台只需新 host。

## D-002 即时模式 + 显式矩形

**背景**:保留模式(retained)需要完整的树 diff/失效机制,v0.1 写不正确。
**决策**:每帧 `app.ui(ctx)` 从头构建;`Column/Row/form_row` 提供布局;控件状态或存调用方(开关、滑条值)或按 Id 存 `Memory`(编辑框、滚动)。
**后果**:API 面小、无失效 bug;代价是每帧重建显示列表(当前规模微不足道)。

## D-003 交互按上一帧几何解析

**背景**:即时模式里,控件在帧内按绘制顺序执行;同帧后画者遮挡先画者,但先画者已经跑完。
**决策**:悬停与点击用 `memory.prev_rects`(上一帧注册的矩形,后注册者为顶层);指针在按下时捕获到控件 id,抬起帧仍归它,`end()` 后释放。`drag_delta` 由 `end()` 记录的上一帧指针位置计算。
**后果**:点击永远命中用户看到的控件;滑条在按下帧即生效;结构变化有一帧悬停延迟(与 egui 同类取舍)。

## D-004 CPU 软件光栅化 + WGL 只做呈现

**背景**:需要确定性的渲染证据(跨构建一致的帧哈希)与最小的后端面积。
**决策**:核心把显示列表光栅化到 RGBA8 帧缓冲(轴对齐矩形解析覆盖率、圆角/描边/线段 SDF 2×2 超采样、字形 alpha 直通);host 用兼容上下文 GL 1.1(纹理 + 三角带)整帧上传,`wglSwapIntervalEXT(1)` 垂直同步。
**后果**:渲染是纯函数,`--smoke` 的 `fb_hash` 在 debug/release 间一致;性能是纯 CPU 的,640×360 release 均帧 ~10ms(v0.1 基线,含 vsync 等待),优化空间在 SIMD 覆盖率与脏矩形(见 roadmap)。

## D-005 字形:放弃 GGO,走系统文本管线

**背景**:初版用 `GetGlyphOutlineW`(GGO_GRAY8)。实测当前 Windows 的 Segoe UI 是可变字体(variable font),GGO 对其返回 GDI_ERROR,拉丁字符全部拿不到(仅 Segoe UI Symbol 成功)——这是 GGO 的已知局限。
**决策**:把字符用 `ExtTextOutW` 画到 32bpp 顶朝下 DIB(白字黑底),红色通道即灰度覆盖率(内存 DC 无 ClearType,正好);步进取 `GetCharWidth32W`,度量取 `GetTextMetricsW`;按 ["Segoe UI", "Microsoft YaHei UI", "Segoe UI Symbol"] 逐族回退,无墨迹即视为缺字换下一族;全失败缓存 tofu。
**后果**:中英混排、空白、符号全部工作;非 BMP 字符 v0.1 不支持(见 roadmap)。

## D-006 imm32 运行时解析

**背景**:GNU 工具链不随附 imm32 导入库,`#[link(name="imm32")]` 链接失败。
**决策**:`GetProcAddress` 在首次使用时解析五个 IMM 入口点(`OnceLock` 缓存),缺失时 IME 相关路径安全降级为无操作。
**后果**:跨 MinGW/MSVC 工具链都能链接;少一个链接期依赖。

## D-007 工具链:钉 GNU 宿主

**背景**:本机无 VS/MSVC 链接器(`link.exe` 实为 GNU coreutils 的同名工具);rimely 的解法是 lld-link + xwin SDK,对独立 GUI 项目过重。
**决策**:`rust-toolchain.toml` 钉 `stable-x86_64-pc-windows-gnu`(自带 MinGW 链接器与导入库);代码不依赖特定 target,CI 上 MSVC 默认工具链同样构建。
**后果**:克隆即建;MSVC 特有路径未在本机验证(CI 覆盖)。

## D-008 证据文化

**背景**:借鉴 rimely 的 receipts 流程:每个里程碑留可复核的证据。
**决策**:`--smoke` 输出 JSON 统计行 + PNG 凭证;证据落 `docs/receipts/`,记录命令与原始输出。
**后果**:`docs/receipts/FOUNDATION-20261009.md` 可完整复核 v0.1 基线。

## D-009 外部复核修复批次(2026-10-09,独立复现驱动)

外部复核(Codex)在基线全绿之外复现了五个问题;本批逐一修复并以回归测试锁定。

1. **嵌套裁剪越界**:`PopClip` 曾恢复为"全屏"而非上一层裁剪,滚动区内的裁剪绘制(编辑框、标签)会溢出到容器外。修复:渲染器每层维护真正的裁剪栈。像素级证据:修复前 smoke 哈希 `143abd386ea1d737`,修复后 `824ec1f60bd9688c`,差异即溢出面板的列表文字;修复后面板外越界采样为 0。
2. **同帧快速点击丢失**:`interact` 在按下帧读到的捕获是设置前的旧值,同一帧内 press+release 的完整点击完全丢失。修复:本帧新建捕获(`drag_started`)与持有捕获等权;`end()` 仅在"释放且帧末未按下"时清除捕获,press-release-press 的连续拖拽不断裂。
3. **动态文字缓存无界**:逐帧变化的字符串(计数、fps)让 layout 缓存永久增长。修复:缓存条目记录最后使用帧,超过 1024 条时先淘汰 600 帧未用的,再退化淘汰最旧;`Ctx::begin` 推进 `Shaper` 帧计数。
4. **资源释放**:GL 上下文与上传纹理从不删除(`WglPresent::Drop` 现在解绑后删除两者)、DIB 位图句柄泄漏(随 `Target` 释放)、窗口类不注销(`UnregisterClassW`)。GL 释放在 DC 释放前显式执行。
5. **字体配置**:`GdiFontSet::with_families` 从空壳变为真实现,字体族链可配置并带 `families()` 查询;族遍历长度不再绑死全局常量。

## D-010 双击检测与按钮 API 演进(2026-10-10,由 T-002/R-002 引入)

**背景**:双击需要调用方同时拿到"单击"与"双击"两个信号,`button -> bool` 表达不了。
**决策**:`Response` 增加 `double_clicked`(与上次完整点击间隔落在 `[0,500)` ms 且位移 < 6 逻辑像素);`FrameInput.time_ms` 由 host 注入,core 不读时钟,测试用注入时间保持确定性;`widgets::button` 返回 `Response`(`activated()` 语义不变,Enter 仅置 `clicked`)。点击配对记录全局存于 `Memory::last_click`。
**备注(已知边界,后续可选跟进)**:配对不按控件 id 隔离;三连击链式配对(第三下与第二下再成双击)。
