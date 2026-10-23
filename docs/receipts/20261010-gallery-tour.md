# gallery 控件巡礼凭证（T-005 / R-005）
- 日期: 2026-10-10
- 仓库: mullion @ 9ee56dc 的工作区（本凭证尚未提交）
- 机器: Windows 10.0.26200
- 说明: UI 变化会改变帧哈希。上一基线 `1dfb43e5d3d74255`（`docs/receipts/EGUI-PIVOT-20261010.md`）不再适用。

## fmt
命令: `cargo fmt --all -- --check`

输出: （空）

退出码: 0

## clippy
命令: `cargo clippy --workspace --all-targets -- -D warnings`

```text
    Checking mullion-host v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion-host)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.26s
```

退出码: 0

## test
命令: `cargo test --workspace`

```text
   Compiling mullion-host v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion-host)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.19s
     Running unittests src\lib.rs (target\x86_64-pc-windows-msvc\debug\deps\mullion_host-4bad1d8ea35452bd.exe)

running 7 tests
test input_map::tests::maps_known_keys ... ok
test input_map::tests::extracts_signed_coords ... ok
test input_map::tests::cursor_icons_map_to_idc ... ok
test png::tests::crc32_known_vector ... ok
test png::tests::adler32_known_vector ... ok
test png::tests::zlib_stored_roundtrip_length ... ok
test png::tests::png_structure ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests mullion_host

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

退出码: 0

合计: 7 passed，0 failed，0 ignored。

## smoke
命令: `cargo run -p mullion-host --example gallery --release -- --smoke docs/receipts/gallery-smoke-t005.png`

```text
   Compiling mullion-host v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion-host)
    Finished `release` profile [optimized] target(s) in 9.45s
     Running `target\x86_64-pc-windows-msvc\release\examples\gallery.exe --smoke docs/receipts/gallery-smoke-t005.png`
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":9.426,"opaque_pixels":230400,"fb_hash":"f98415a03c8f90c5"}
```

退出码: 0

## 像素复核（docs/receipts/gallery-smoke-t005.png）
PNG 解码后逐像素统计:

- 尺寸: 640×360
- 不透明: 230400/230400
- 背景接近 (27,27,27)（各通道差 ≤ 2）: 198580
- 强调色接近 (79,140,255)（R±8、G±12、B±8）: 2498
- 文字墨迹（不透明、灰度 140–230、通道互差 ≤ 12）: 2037
