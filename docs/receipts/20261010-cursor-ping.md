# Cursor 握手凭证（T-001 / R-001）
- 日期: 2026-10-10
- 仓库: mullion @ 85f26e7（工作区，尚未把本凭证提交）
- 机器: Windows 10.0.26200
- 工具链: cargo 1.99.0 (5f94df478 2026-08-27)；rustc 1.99.0 (b940084d7 2026-09-28)
- 执行方: Cursor 编辑器内的 agent（模型 Grok 4.7）

## fmt
命令: `cargo fmt --all -- --check`

输出: （空）

退出码: 0

## clippy
命令: `cargo clippy --workspace --all-targets -- -D warnings`

```text
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.09s
```

退出码: 0

## test
命令: `cargo test --workspace`

```text
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.43s
     Running unittests src\lib.rs (target\debug\deps\mullion-6165feee2456890f.exe)

running 53 tests
test color::tests::coverage_scales_alpha ... ok
test color::tests::hex_parsing ... ok
test color::tests::half_blend_is_midpoint ... ok
test geometry::tests::clamp_keeps_inside ... ok
test context::tests::drag_delta_reports_movement ... ok
test context::tests::click_outside_on_release_is_not_clicked ... ok
test context::tests::click_requires_hover_on_press_and_release ... ok
test color::tests::opaque_src_replaces ... ok
test context::tests::later_rect_steals_hover ... ok
test context::tests::press_release_press_keeps_dragging ... ok
test context::tests::same_frame_click_registers ... ok
test context::tests::tab_cycles_focus ... ok
test color::tests::transparent_src_keeps_dst ... ok
test geometry::tests::intersect_basics ... ok
test context::tests::consumed_key_is_not_seen_twice ... ok
test layout::tests::form_row_split ... ok
test geometry::tests::translate_and_invert ... ok
test layout::tests::column_splits_with_spacing ... ok
test layout::tests::row_and_padding ... ok
test memory::tests::backspace_char_and_multibyte ... ok
test geometry::tests::contains_is_half_open ... ok
test memory::tests::backspace_removes_selection ... ok
test memory::tests::caret_movement_clamps_to_boundaries ... ok
test memory::tests::cut_removes_and_returns ... ok
test memory::tests::delete_forward_works ... ok
test memory::tests::id_of_is_stable ... ok
test memory::tests::insert_drops_newlines ... ok
test memory::tests::insert_replaces_selection ... ok
test memory::tests::select_all_and_copy_slice ... ok
test memory::tests::shift_selection_extends_then_collapses ... ok
test render::tests::fill_rect_full_and_partial_coverage ... ok
test render::tests::fill_rect_respects_clip ... ok
test render::tests::line_draws_horizontal ... ok
test render::tests::nested_clip_restores_previous_clip ... ok
test render::tests::stroke_only_draws_border ... ok
test style::tests::themes_differ ... ok
test render::tests::rounded_rect_interior_filled_corner_not ... ok
test render::tests::text_blits_alpha ... ok
test style::tests::scale_multiplication ... ok
test text::tests::baseline_centers_glyph_run ... ok
test text::tests::hit_multibyte_is_char_aligned ... ok
test text::tests::caret_and_hit_roundtrip ... ok
test text::tests::missing_glyph_becomes_tofu_and_is_cached ... ok
test text::tests::layout_widths_and_caching ... ok
test widgets::tests::line_edit_select_all_copy_and_paste ... ok
test widgets::tests::line_edit_types_and_edits ... ok
test widgets::tests::toggle_flips_on_click ... ok
test widgets::tests::window_close_button_closes ... ok
test widgets::tests::scroll_area_clamps_and_reports_offset ... ok
test widgets::tests::window_drag_moves_position ... ok
test render::tests::render_is_deterministic ... ok
test text::tests::layout_cache_evicts_stale_entries ... ok
test text::tests::layout_cache_is_bounded_for_dynamic_text ... ok

test result: ok. 53 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

     Running tests\frame.rs (target\debug\deps\frame-a5df184c5b702099.exe)

running 2 tests
test interaction_updates_frame_output ... ok
test full_frame_renders_deterministically ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.05s

     Running unittests src\lib.rs (target\debug\deps\mullion_host-fd906c86992e091f.exe)

running 7 tests
test input_map::tests::extracts_signed_coords ... ok
test input_map::tests::maps_known_keys ... ok
test png::tests::adler32_known_vector ... ok
test png::tests::zlib_stored_roundtrip_length ... ok
test png::tests::crc32_known_vector ... ok
test png::tests::png_structure ... ok
test gdi_font::tests::gdi_metrics_and_glyph ... ok

test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

   Doc-tests mullion

running 1 test
test crates\mullion\src\lib.rs - (line 10) ... ignored

test result: ok. 0 passed; 0 failed; 1 ignored; 0 measured; 0 filtered out; finished in 0.00s

all doctests ran in 2.29s; merged doctests compilation took 0.64s
   Doc-tests mullion_host

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

合计: 62 passed，0 failed，1 ignored（`mullion` 文档测试）。

## 能力
- 能读 `docs/` 与仓库文件。已读 `AGENTS.md`、`docs/coordination/PROTOCOL.md`、三个信箱。
- 能在仓库根运行 cargo。fmt / clippy / test 均退出码 0。
- 能改文件。本握手只改协调文档和本凭证，未改 crate 代码。
- PATH 上有 `cursor.cmd`，没有名为 `cursor-agent` 的命令。
- `wsl.exe` 在系统里存在；这次探测调用挂起后被停掉，没有可用的发行版输出。
