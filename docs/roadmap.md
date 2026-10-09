# 路线图

v0.1 基线之后的候选项,按价值/成本排序;不是承诺。

## 文本
- 多行文本编辑(软换行、垂直滚动)——`EditState` 已按纯状态机设计,扩展不破坏现有测试。
- 文本换行布局(label/按钮内自动换行)。
- 非 BMP 字符(emoji):GDI 路线需要 UTF-16 代理对处理;或评估 DirectWrite 后端。
- 字距/kerning:`GetKerningPairsW` 或改用 DirectWrite shaping;拉丁观感提升明显。

## 光栅化性能
- 脏矩形/分层失效:整帧重绘在 release 下 640×360 ~10ms;1080p 全屏约 6 倍面积,需要分层缓存。
- SIMD 覆盖率(fill_rect 已是解析覆盖率,瓶颈在逐像素 blend;可按行批量)。
- 行缓存友好的字形合成(当前逐字形逐像素 alpha blend)。

## 控件与交互
- 双击(时间戳判定)、右键上下文菜单、下拉框、标签页。
- 窗口 z 序:点击置顶(当前按绘制顺序固定)。
- 键盘导航完整化(方向键在控件间移动焦点;目前 Tab/Shift+Tab 已可用)。

## 平台
- 后端抽象化:`Host` trait + `run()` 泛型化,便于第二个后端(winit/wayland)。
- 高帧率路径:v-sync 外的节流策略与忙等消除。
- 多窗口(每窗口一个 GL 上下文与 Runner)。

## 工程化
- crates.io 发布(发布前需确认 crate 名可用)。
- 基准套件(criterion 或手写帧时间采集)防性能回归。
- 文档站与 doctest 示例。
