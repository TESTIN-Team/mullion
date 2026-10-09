# mullion

窗棂——窗户的横竖框。一个纯 Rust 的即时模式(immediate-mode)GUI 框架:核心 crate 零依赖、完全可无窗口单测;Windows 后端用 Win32 + WGL 呈现,GDI 光栅化字形(含中文),支持按显示器 DPI、IME 输入和剪贴板。

```
┌───────────────────────── mullion workspace ─────────────────────────┐
│  crates/mullion        纯 Rust、零依赖:UI 核心                       │
│    geometry/color/style  几何、直通 alpha 混色、主题与间距             │
│    text                  字体抽象 + 字形缓存 + 排版/命中测试           │
│    input/memory          帧输入事件、控件跨帧状态(编辑框纯状态机)      │
│    context               交互解析(悬停/点击/拖拽/焦点)与绘制列表      │
│    layout/widgets        Column/Row、button/toggle/slider/line_edit  │
│                         /window/scroll_area …                        │
│    render                CPU 软件光栅化(解析 AA + SDF 圆角 + 字形)    │
│  crates/mullion-host    Windows 后端                                  │
│    window                Win32 窗口、消息泵、DPI、IME、主循环          │
│    wgl                   WGL 兼容上下文,帧缓冲纹理上传呈现            │
│    gdi_font              GDI 字形光栅化(多字体族回退,可变字体安全)     │
│    clipboard/png         CF_UNICODETEXT 剪贴板、零依赖 PNG 写出        │
└──────────────────────────────────────────────────────────────────────┘
```

## 快速开始

本仓库钉住 GNU 宿主工具链(本机无 MSVC 链接器也能构建);代码与目标无关,CI 上默认 MSVC 工具链同样通过。

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p mullion-host --example gallery --release
```

gallery 展示全部控件:按钮、开关、复选、滑条、进度条、滚动列表、单行文本编辑(选区、Ctrl+A/C/V/X、IME 组合态)、可拖动窗口、明暗主题切换。

## 无头验证(--smoke)

```text
cargo run -p mullion-host --example gallery --release -- --smoke docs/receipts/gallery-smoke.png
```

隐藏窗口渲染 12 帧,第 6 帧把窗口从 1000×640 缩放到 640×360(覆盖帧缓冲重建路径),末帧写出 PNG 凭证并向 stdout 打一行 JSON 统计。基线证据与逐次验证记录在 `docs/receipts/`。

## 设计要点(详见 docs/decisions.md)

- **交互用上一帧的几何**:悬停/点击按上一帧注册的矩形解析,后画的控件在顶层;指针按下即捕获、抬起释放,滑条与选区拖拽因此稳定。
- **软件光栅化作为核心能力**:显示列表到 RGBA 帧缓冲是纯函数,同输入必同输出(smoke 的 `fb_hash` 在 debug/release 间一致);后端只负责把帧缓冲贴上屏幕。
- **字形走系统文本管线**:`GetGlyphOutlineW` 在可变字体(Segel UI 变体、当前 Windows 的默认 UI 字体)上直接失败,因此改用 `ExtTextOutW` 画到 DIB、取红色通道为覆盖率,配合 `GetCharWidth32W` 步进;多字体族回退让中英混排落在正确字体上。
- **IME**:组合串与提交串经 `ImmGetCompositionStringW` 进入帧输入,组合态在光标处内联显示(下划线+组合光标),组合窗口与候选窗每帧跟随光标;`imm32` 无 MinGW 导入库,入口点用 `GetProcAddress` 运行时解析。

## 状态

v0.1 基线:核心 50 项测试(几何/混色/编辑状态机/交互两帧解析/光栅化覆盖/整帧确定性)+ 后端 7 项(PNG/CRC/Adler/键映射/字体)全绿;`cargo clippy --workspace --all-targets -- -D warnings` 零告警。已知边界与下一步见 `docs/roadmap.md`。

## 许可

MIT OR Apache-2.0,见 `LICENSE-MIT` / `LICENSE-APACHE`。
