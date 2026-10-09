# mullion 复核修复批次凭证(REVIEW-FIXES)
- 日期:2026-10-09
- 范围:嵌套裁剪 / 同帧点击 / 动态文字缓存 / 资源释放 / 字体配置(详见 docs/decisions.md D-009)

## fmt
通过(无输出)

## clippy(-D warnings)
错误数 0(通过)

## test
test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.06s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

## smoke(release,修复后)
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":9.509,"opaque_pixels":230400,"fb_hash":"824ec1f60bd9688c"}

## 裁剪修复像素证据(docs/receipts/gallery-smoke.png)
- 帧哈希 143abd386ea1d737(修复前)→ 824ec1f60bd9688c(修复后):差异即此前溢出面板的滚动列表文字
- 修复后面板下方越界像素采样:0/1240;面板内列表文字墨迹采样:76

## 新增回归测试
- render::nested_clip_restores_previous_clip
- context::same_frame_click_registers / press_release_press_keeps_dragging
- text::layout_cache_is_bounded_for_dynamic_text / layout_cache_evicts_stale_entries
