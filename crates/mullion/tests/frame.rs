//! Headless full-frame test: build a UI with widgets, render it, and assert
//! determinism plus a few known pixels. This exercises the whole core stack
//! (ctx -> widgets -> display list -> rasterizer) without a host.

use mullion::input::FrameInput;
use mullion::memory::id_of;
use mullion::render::Framebuffer;
use mullion::{Ctx, Vec2, render, widgets};
use std::sync::Arc;

struct BoxFont;
impl mullion::FontBackend for BoxFont {
    fn metrics(&self, _px: f32) -> mullion::FontMetrics {
        mullion::FontMetrics {
            ascent: 11.0,
            descent: 3.0,
            line_gap: 0.0,
        }
    }
    fn glyph(&self, ch: char, px: f32) -> Option<mullion::Glyph> {
        Some(mullion::Glyph {
            ch,
            px,
            width: 5,
            height: 10,
            left: 0,
            top: -8,
            advance: 6.0,
            alpha: vec![230; 50],
        })
    }
}

fn build_frame(ctx: &mut Ctx, input: FrameInput) {
    ctx.begin(input);

    let mut open = true;
    let mut pos = Vec2::new(8.0, 8.0);
    if let Some(body) = widgets::window(
        ctx,
        id_of("main"),
        "Gallery",
        &mut pos,
        Vec2::new(280.0, 180.0),
        &mut open,
    ) {
        let mut col = mullion::layout::Column::with_padding(body, 8.0, 6.0);
        let r = col.add(28.0);
        widgets::button(ctx, r, "按钮");
        let r = col.add(20.0);
        widgets::label(ctx, r, "标签文本", None);
        let r = col.add(24.0);
        widgets::slider(ctx, r, id_of("sl"), 0.4, 0.0, 1.0);
        let r = col.add(26.0);
        widgets::line_edit(ctx, r, id_of("ed"), "输入…");
    }

    ctx.end();
}

#[test]
fn full_frame_renders_deterministically() {
    let mut a = Ctx::new(Arc::new(BoxFont));
    let mut b = Ctx::new(Arc::new(BoxFont));

    // Prime interaction rects, then a real frame.
    for ctx in [&mut a, &mut b] {
        build_frame(ctx, FrameInput::default());
    }

    let mut fb_a = Framebuffer::new(320, 220);
    let mut fb_b = Framebuffer::new(320, 220);
    build_frame(&mut a, FrameInput::default());
    render(&a.display, &a.shaper, &mut fb_a);
    build_frame(&mut b, FrameInput::default());
    render(&b.display, &b.shaper, &mut fb_b);

    assert_eq!(
        fb_a.hash(),
        fb_b.hash(),
        "identical inputs must render identically"
    );
    assert!(fb_a.opaque_pixels() > 1000, "frame must not be empty");

    // Background outside the window.
    let bg = a.style.theme.window_bg;
    assert_eq!(fb_a.pixel(310, 210), bg);
    // Window surface near the center of the panel body (drawn over the bg).
    let th = a.style.theme;
    let surface = th.surface;
    let p = fb_a.pixel(150, 60);
    assert_eq!(
        p, surface,
        "panel body must show the surface color, got {p:?}"
    );
}

#[test]
fn interaction_updates_frame_output() {
    let mut ctx = Ctx::new(Arc::new(BoxFont));

    // Prime.
    build_frame(&mut ctx, FrameInput::default());
    build_frame(&mut ctx, FrameInput::default());

    // Click the line edit (4th row: y = 8 + 32(title) + 4 + 8(pad) + 28+6+20+6+24+6 = ...).
    // Compute from layout constants: pad=8, gap=6, title=32, title pad 4.
    let y = 8.0 + 32.0 + 4.0 + 8.0 + (28.0 + 6.0) + (20.0 + 6.0) + (24.0 + 6.0) + 13.0;
    let input = FrameInput {
        mouse_pos: Some(Vec2::new(40.0, y)),
        primary_pressed: true,
        primary_down: true,
        ..Default::default()
    };
    build_frame(&mut ctx, input);
    build_frame(
        &mut ctx,
        FrameInput {
            mouse_pos: Some(Vec2::new(40.0, y)),
            primary_released: true,
            ..Default::default()
        },
    );

    let ed = id_of("ed");
    assert_eq!(ctx.memory.focus, Some(ed), "click must focus the line edit");

    // Typing lands in the buffer.
    let fi = FrameInput {
        text: "你好mullion".into(),
        ..Default::default()
    };
    build_frame(&mut ctx, fi);
    assert_eq!(ctx.memory.edit_state(ed).text(), "你好mullion");
}
