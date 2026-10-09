//! mullion widget gallery. Runs interactively by default; `--smoke [out.png]`
//! renders 12 hidden frames (resizing midway), writes a PNG receipt and a
//! JSON stats line, then exits.

use mullion::layout::{Column, form_row};
use mullion::memory::id_of;
use mullion::widgets;
use mullion::{Ctx, Vec2};
use mullion_host::{App, SmokeConfig, WindowConfig, run};
use std::time::Instant;

struct Gallery {
    toggle: bool,
    check_a: bool,
    check_b: bool,
    slider: f32,
    counter: u32,
    win_main: Vec2,
    win_input: Vec2,
    win_about: Vec2,
    input_open: bool,
    about_open: bool,
    t0: Instant,
    frames: u64,
    fps: f32,
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            toggle: true,
            check_a: true,
            check_b: false,
            slider: 0.35,
            counter: 0,
            win_main: Vec2::new(24.0, 24.0),
            win_input: Vec2::new(360.0, 24.0),
            win_about: Vec2::new(360.0, 300.0),
            input_open: true,
            about_open: true,
            t0: Instant::now(),
            frames: 0,
            fps: 0.0,
        }
    }
}

impl App for Gallery {
    fn ui(&mut self, ctx: &mut Ctx) {
        // Framerate over one-second windows.
        self.frames += 1;
        let elapsed = self.t0.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.frames as f32 / elapsed;
            self.frames = 0;
            self.t0 = Instant::now();
        }

        if let Some(body) = widgets::window(
            ctx,
            id_of("main"),
            "控件总览",
            &mut self.win_main,
            Vec2::new(320.0, 300.0),
            &mut true,
        ) {
            let mut col = Column::with_padding(body, 10.0, 8.0);

            let r = col.add(30.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "按钮", None);
            if widgets::button(ctx, fr, "点我") {
                self.counter = self.counter.wrapping_add(1);
            }

            let r = col.add(30.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "计数", None);
            let count_text = format!("已点击 {} 次", self.counter);
            widgets::label(ctx, fr, &count_text, None);

            let r = col.add(26.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "开关", None);
            self.toggle = widgets::toggle(ctx, fr, self.toggle);

            let r = col.add(24.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "复选", None);
            let mut row = mullion::layout::Row::new(fr, 8.0);
            let b = row.add(120.0);
            self.check_a = widgets::checkbox(ctx, b, self.check_a);
            let b = row.add(120.0);
            self.check_b = widgets::checkbox(ctx, b, self.check_b);

            let r = col.add(40.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "滑条", None);
            self.slider = widgets::slider(ctx, fr, id_of("slider"), self.slider, 0.0, 100.0);

            let r = col.add(24.0);
            let (lr, fr) = form_row(r, 64.0);
            widgets::label(ctx, lr, "数值", None);
            let v = format!("{:.1}", self.slider);
            widgets::label(ctx, fr, &v, None);

            let r = col.add(8.0);
            widgets::progress(ctx, r, self.slider / 100.0);

            let r = col.add(4.0);
            widgets::separator(ctx, r);

            let rest = col.remaining_rect();
            widgets::scroll_area(ctx, id_of("scroll"), rest, 220.0, |ctx, content| {
                let mut list = Column::new(content, 4.0);
                for i in 0..12 {
                    let row = list.add(22.0);
                    let text = format!("列表项 {:02} — mullion 软件光栅化", i + 1);
                    widgets::label(ctx, row, &text, None);
                }
            });
        }

        if self.input_open {
            if let Some(body) = widgets::window(
                ctx,
                id_of("input"),
                "文本输入 / IME",
                &mut self.win_input,
                Vec2::new(320.0, 150.0),
                &mut self.input_open,
            ) {
                let mut col = Column::with_padding(body, 10.0, 8.0);
                let r = col.add(28.0);
                let name = widgets::line_edit(ctx, r, id_of("name"), "输入你的名字…");
                let r = col.add(24.0);
                let hello = if name.trim().is_empty() {
                    "你好,匿名用户".to_string()
                } else {
                    format!("你好,{}", name.trim())
                };
                widgets::label(ctx, r, &hello, None);
                let r = col.add(8.0);
                widgets::separator(ctx, r);
                let r = col.add(24.0);
                widgets::label(ctx, r, "支持中文 IME、选区、Ctrl+A/C/V/X", None);
            }
        }

        if self.about_open {
            if let Some(body) = widgets::window(
                ctx,
                id_of("about"),
                "关于",
                &mut self.win_about,
                Vec2::new(320.0, 190.0),
                &mut self.about_open,
            ) {
                let mut col = Column::with_padding(body, 10.0, 6.0);
                let r = col.add(24.0);
                widgets::label(ctx, r, "mullion v0.1 — 纯 Rust GUI 框架", None);
                let r = col.add(24.0);
                widgets::label(ctx, r, "核心零依赖,Windows 后端 Win32+WGL", None);
                let r = col.add(24.0);
                let stats = format!("FPS {:.0} · 缩放 {:.2}", self.fps, ctx.style.scale);
                widgets::label(ctx, r, &stats, None);
                let r = col.add(30.0);
                if widgets::button(ctx, r, "切换主题") {
                    ctx.style.theme = if ctx.style.theme == mullion::Theme::dark() {
                        mullion::Theme::light()
                    } else {
                        mullion::Theme::dark()
                    };
                }
            }
        }
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config = WindowConfig {
        title: "mullion · 控件展示".into(),
        size: (1000, 640),
        visible: true,
        smoke: None,
    };

    if let Some(pos) = args.iter().position(|a| a == "--smoke") {
        let png_path = args
            .get(pos + 1)
            .filter(|s| !s.starts_with('-'))
            .map(std::path::PathBuf::from);
        config.visible = false;
        config.size = (1000, 640);
        config.smoke = Some(SmokeConfig {
            frames: 12,
            resize_at: 6,
            resize_to: (640, 360),
            png_path,
        });
    }

    let stats = run(Box::new(Gallery::default()), config).expect("window creation failed");
    println!(
        "{{\"frames\":{},\"width\":{},\"height\":{},\"scale\":{:.3},\"avg_frame_ms\":{:.3},\"opaque_pixels\":{},\"fb_hash\":\"{:016x}\"}}",
        stats.frames,
        stats.width,
        stats.height,
        stats.scale,
        stats.avg_frame_ms,
        stats.opaque_pixels,
        stats.fb_hash
    );
}
