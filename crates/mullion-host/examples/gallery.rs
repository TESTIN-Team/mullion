//! mullion + egui widget gallery. Interactive by default; `--smoke [out.png]`
//! renders 12 hidden frames (resizing midway), captures the backbuffer as a
//! PNG receipt, prints a JSON stats line and exits.

use mullion_host::{SmokeConfig, WindowConfig, run_egui};
use std::time::Instant;

struct Gallery {
    toggle: bool,
    check_a: bool,
    check_b: bool,
    slider: f32,
    counter: u32,
    name: String,
    combo: usize,
    dark: bool,
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
            slider: 35.0,
            counter: 0,
            name: String::new(),
            combo: 0,
            dark: true,
            t0: Instant::now(),
            frames: 0,
            fps: 0.0,
        }
    }
}

const THEMES: &[&str] = &["强调色 · 蓝", "强调色 · 绿", "强调色 · 橙"];
const ACCENTS: [egui::Color32; 3] = [
    egui::Color32::from_rgb(79, 140, 255),
    egui::Color32::from_rgb(94, 190, 126),
    egui::Color32::from_rgb(240, 160, 64),
];

impl Gallery {
    fn ui(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        let elapsed = self.t0.elapsed().as_secs_f32();
        if elapsed >= 1.0 {
            self.fps = self.frames as f32 / elapsed;
            self.frames = 0;
            self.t0 = Instant::now();
        }

        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.selection.bg_fill = ACCENTS[self.combo.min(2)];
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("mullion · egui 控件总览");
            ui.separator();

            egui::SidePanel::left("main")
                .default_width(340.0)
                .show_inside(ui, |ui| {
                    egui::Grid::new("form")
                        .num_columns(2)
                        .spacing([12.0, 8.0])
                        .show(ui, |ui| {
                            ui.label("按钮");
                            if ui.button("点我(单击 +1)").clicked() {
                                self.counter = self.counter.wrapping_add(1);
                            }
                            ui.end_row();

                            ui.label("双击");
                            if ui.button("点我(双击 +10)").double_clicked() {
                                self.counter = self.counter.wrapping_add(10);
                            }
                            ui.end_row();

                            ui.label("计数");
                            ui.label(format!("已点击 {} 次", self.counter));
                            ui.end_row();

                            ui.label("开关");
                            ui.toggle_value(&mut self.toggle, "开关");
                            ui.end_row();

                            ui.label("复选");
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut self.check_a, "选项 A");
                                ui.checkbox(&mut self.check_b, "选项 B");
                            });
                            ui.end_row();

                            ui.label("滑条");
                            let slider =
                                egui::Slider::new(&mut self.slider, 0.0..=100.0).text("数值");
                            ui.add(slider).on_hover_text("拖动调整数值,方向键微调");
                            ui.end_row();

                            ui.label("进度");
                            ui.add(egui::ProgressBar::new(self.slider / 100.0).show_percentage());
                            ui.end_row();

                            ui.label("下拉");
                            egui::ComboBox::from_id_salt("combo")
                                .selected_text(THEMES[self.combo])
                                .width(160.0)
                                .show_ui(ui, |ui| {
                                    for (i, name) in THEMES.iter().enumerate() {
                                        ui.selectable_value(&mut self.combo, i, *name);
                                    }
                                });
                            ui.end_row();
                        });

                    ui.separator();
                    ui.label("滚动列表");
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for i in 0..12 {
                            ui.label(format!("列表项 {:02} — mullion × egui · WGL 呈现", i + 1));
                        }
                    });
                });

            egui::CentralPanel::default().show_inside(ui, |ui| {
                egui::Grid::new("input")
                    .num_columns(2)
                    .spacing([12.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("文本输入");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.name).hint_text("输入你的名字…"),
                        );
                        ui.end_row();

                        ui.label("问候");
                        let hello = if self.name.trim().is_empty() {
                            "你好,匿名用户".to_string()
                        } else {
                            format!("你好,{}", self.name.trim())
                        };
                        ui.label(hello);
                        ui.end_row();
                    });

                ui.separator();
                ui.label("支持中文 IME、选区、Ctrl+A/C/V/X(egui 内建)");
                ui.label("悬停控件查看 tooltip;滑条支持方向键微调");

                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.separator();
                    ui.horizontal(|ui| {
                        if ui.button("切换明暗").clicked() {
                            self.dark = !self.dark;
                        }
                        ui.label(format!("FPS {:.0} · egui 0.32 · Win32 + WGL", self.fps));
                    });
                });
            });
        });
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut config = WindowConfig {
        title: "mullion · egui 控件展示".into(),
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

    let mut gallery = Gallery::default();
    let stats =
        run_egui(Box::new(move |ctx| gallery.ui(ctx)), config).expect("window creation failed");
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
