# mullion UI 层切换 egui 凭证(EGUI-PIVOT)
- 日期:2026-10-10;决策 D-012;T-004 取消,T-005 已派

## fmt
通过(无输出)

## clippy(-D warnings)
错误数 0(0=通过)

## test
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

## smoke(release)
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":9.543,"opaque_pixels":230400,"fb_hash":"1dfb43e5d3d74255"}

## smoke(debug)
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":11.845,"opaque_pixels":230400,"fb_hash":"1dfb43e5d3d74255"}

## 确定性与像素复核(docs/receipts/gallery-smoke-egui.png)
- debug 与 release fb_hash 一致:1dfb43e5d3d74255(GL 光栅化同机确定)
- 640×360 全帧 230400/230400 不透明;背景 egui dark (27,27,27) 主导;强调色 (79,140,255) 在场
- 标题区文字墨迹 394 采样点(中灰,egui dark 文本色),含宽幅中文字形(ASCII 复核确认为真实笔画非豆腐块)
- 可见窗口交互启动 4s 无崩溃(taskkill 正常终止)
