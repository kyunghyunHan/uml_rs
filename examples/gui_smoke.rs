//! Native renderer smoke test. Screenshot is for UI inspection, never diagram export.
use eframe::egui;
use uml_studio::app::{UmlApp, example_project};
struct Smoke {
    app: UmlApp,
    frames: usize,
    output: std::path::PathBuf,
}
impl eframe::App for Smoke {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let capture = ctx.input(|i| {
            i.events.iter().find_map(|e| {
                if let egui::Event::Screenshot { image, .. } = e {
                    Some(image.clone())
                } else {
                    None
                }
            })
        });
        if let Some(image) = capture {
            let data: Vec<u8> = image.pixels.iter().flat_map(|p| p.to_array()).collect();
            let pixmap = resvg::tiny_skia::Pixmap::from_vec(
                data,
                resvg::tiny_skia::IntSize::from_wh(image.width() as u32, image.height() as u32)
                    .unwrap(),
            )
            .unwrap();
            pixmap.save_png(&self.output).unwrap();
            println!(
                "GUI rendered: {} ({}x{})",
                self.output.display(),
                image.width(),
                image.height()
            );
            self.app.project = None;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        self.app.update(ctx, frame);
        self.frames += 1;
        if self.frames == 4 {
            self.app.fit_active();
        }
        if self.frames == 8 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        if self.frames > 120 {
            panic!("Native screenshot timed out");
        }
        ctx.request_repaint();
    }
}
fn main() -> eframe::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let output = args
        .get(1)
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| "/tmp/uml-studio-gui.png".into());
    let index: usize = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
    let small = args.iter().any(|a| a == "--small");
    let review = args.iter().any(|a| a == "--review");
    eframe::run_native(
        "UML Studio · GUI smoke",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size(if small {
                [820., 620.]
            } else {
                [1440., 900.]
            }),
            ..Default::default()
        },
        Box::new(move |cc| {
            let mut app = UmlApp::new(cc);
            let mut p = if review {
                uml_studio::demo::review_project()
            } else {
                example_project()
            };
            let len = p.diagrams.len();
            p.diagrams.rotate_left(index % len);
            app.project = Some(p);
            Ok(Box::new(Smoke {
                app,
                frames: 0,
                output,
            }))
        }),
    )
}
