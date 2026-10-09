# mullion 基线凭证(FOUNDATION)
- 日期:2026-10-09
- 机器:Windows 11 x64(26200),Rust 1.99.0(stable-x86_64-pc-windows-gnu)

## fmt
cargo fmt --all -- --check: 通过(无输出)

## clippy
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.11s

## test
test result: ok. 48 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.07s
test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s
test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

## smoke(debug)
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":128.223,"opaque_pixels":230400,"fb_hash":"143abd386ea1d737"}

## smoke(release)
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":10.573,"opaque_pixels":230400,"fb_hash":"143abd386ea1d737"}

## 确定性
debug 与 release 的 fb_hash 一致:143abd386ea1d737(见上两行 JSON)

## 像素复核(docs/receipts/gallery-smoke.png,python 解码断言)
- 640×360,角落(5,5)=窗口背景色 0x14161a;右下角为关于面板 surface 0x1e2127
- 主面板区域主色:surface 0x1e2127(1440 采样)、标题栏 surface_active 0x2f3440、强调色 0x4f8cff、文字 0xe6e9ee
- 全帧 230400/230400 像素不透明
