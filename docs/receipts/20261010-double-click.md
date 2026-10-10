# 双击检测凭证（T-002 / R-002）
- 日期: 2026-10-10
- 仓库: mullion，基于 e0fb8bd 的工作区（本凭证尚未提交）
- 机器: Windows 10.0.26200
- 工具链: cargo 1.99.0 (5f94df478 2026-08-27)；rustc 1.99.0 (b940084d7 2026-09-28)

## fmt
命令: `cargo fmt --all -- --check`

输出: （空）

退出码: 0

## clippy
命令: `cargo clippy --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.02s
```

退出码: 0

## test
命令: `cargo test --workspace`

```text
   Compiling mullion v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion)
   Compiling mullion-host v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion-host)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 2.12s
     Running unittests src\lib.rs (target\debug\deps\mullion-6165feee2456890f.exe)

running 56 tests
test color::tests::coverage_scales_alpha ... ok
test color::tests::half_blend_is_midpoint ... ok
test color::tests::hex_parsing ... ok
test color::tests::opaque_src_replaces ... ok
test color::tests::transparent_src_keeps_dst ... ok
test context::tests::click_requires_hover_on_press_and_release ... ok
test context::tests::click_outside_on_release_is_not_clicked ... ok
test context::tests::consumed_key_is_not_seen_twice ... ok
test context::tests::double_click_rejected_after_600ms ... ok
test context::tests::double_click_rejected_when_pointer_moves_20px ... ok
test context::tests::double_click_within_400ms ... ok
test context::tests::drag_delta_reports_movement ... ok
test context::tests::press_release_press_keeps_dragging ... ok
test context::tests::same_frame_click_registers ... ok
test context::tests::tab_cycles_focus ... ok
test context::tests::later_rect_steals_hover ... ok
test geometry::tests::clamp_keeps_inside ... ok
test geometry::tests::contains_is_half_open ... ok
test geometry::tests::intersect_basics ... ok
test geometry::tests::translate_and_invert ... ok
test layout::tests::column_splits_with_spacing ... ok
test layout::tests::row_and_padding ... ok
test layout::tests::form_row_split ... ok
test memory::tests::backspace_char_and_multibyte ... ok
test memory::tests::backspace_removes_selection ... ok
test memory::tests::caret_movement_clamps_to_boundaries ... ok
test memory::tests::cut_removes_and_returns ... ok
test memory::tests::delete_forward_works ... ok
test memory::tests::insert_drops_newlines ... ok
test memory::tests::id_of_is_stable ... ok
test memory::tests::insert_replaces_selection ... ok
test memory::tests::select_all_and_copy_slice ... ok
test memory::tests::shift_selection_extends_then_collapses ... ok
test render::tests::fill_rect_full_and_partial_coverage ... ok
test render::tests::fill_rect_respects_clip ... ok
test render::tests::line_draws_horizontal ... ok
test render::tests::nested_clip_restores_previous_clip ... ok
test render::tests::stroke_only_draws_border ... ok
test render::tests::rounded_rect_interior_filled_corner_not ... ok
test style::tests::scale_multiplication ... ok
test render::tests::text_blits_alpha ... ok
test style::tests::themes_differ ... ok
test text::tests::baseline_centers_glyph_run ... ok
test text::tests::caret_and_hit_roundtrip ... ok
test text::tests::hit_multibyte_is_char_aligned ... ok
test text::tests::layout_widths_and_caching ... ok
test text::tests::missing_glyph_becomes_tofu_and_is_cached ... ok
test widgets::tests::line_edit_types_and_edits ... ok
test widgets::tests::scroll_area_clamps_and_reports_offset ... ok
test widgets::tests::line_edit_select_all_copy_and_paste ... ok
test widgets::tests::toggle_flips_on_click ... ok
test widgets::tests::window_close_button_closes ... ok
test widgets::tests::window_drag_moves_position ... ok
test render::tests::render_is_deterministic ... ok
test text::tests::layout_cache_evicts_stale_entries ... ok
test text::tests::layout_cache_is_bounded_for_dynamic_text ... ok

test result: ok. 56 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\frame.rs (target\debug\deps\frame-a5df184c5b702099.exe)

running 2 tests
test interaction_updates_frame_output ... ok
test full_frame_renders_deterministically ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src\lib.rs (target\debug\deps\mullion_host-fd906c86992e091f.exe)

running 7 tests
test input_map::tests::extracts_signed_coords ... ok
test input_map::tests::maps_known_keys ... ok
test png::tests::crc32_known_vector ... ok
test png::tests::adler32_known_vector ... ok
test png::tests::zlib_stored_roundtrip_length ... ok
test png::tests::png_structure ... ok
test gdi_font::tests::gdi_metrics_and_glyph ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

   Doc-tests mullion

running 1 test
test crates\mullion\src\lib.rs - (line 10) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 1.67s; merged doctests compilation took 0.25s
   Doc-tests mullion_host

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

退出码: 0

合计: 65 passed，0 failed，1 ignored。其中新增 3 个：`double_click_within_400ms`、`double_click_rejected_after_600ms`、`double_click_rejected_when_pointer_moves_20px`。

## smoke
命令: `cargo run -p mullion-host --example gallery --release -- --smoke`

```text
   Compiling mullion v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion)
   Compiling mullion-host v0.1.0 (C:\Users\rr\Documents\GitHub\mullion\crates\mullion-host)
    Finished `release` profile [optimized] target(s) in 3.79s
     Running `target\release\examples\gallery.exe --smoke`
{"frames":12,"width":640,"height":360,"scale":1.000,"avg_frame_ms":9.706,"opaque_pixels":230400,"fb_hash":"824ec1f60bd9688c"}
```

退出码: 0

`fb_hash` 为 `824ec1f60bd9688c`，与 `docs/receipts/REVIEW-FIXES-20261009.md` 的 release smoke 逐字相同。`FOUNDATION-20261009.md` 里的 `143abd386ea1d737` 是更早的基线，已被那份修复凭证替换。
