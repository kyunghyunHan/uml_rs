use uml_studio::{app::UmlApp, model::Project, scene, storage};
fn main() -> eframe::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    if args
        .get(1)
        .is_some_and(|a| a == "--example" || a == "--review-example")
    {
        let p = if args[1] == "--review-example" {
            uml_studio::demo::review_project()
        } else {
            uml_studio::app::example_project()
        };
        storage::save(
            std::path::Path::new(
                args.get(2)
                    .map(String::as_str)
                    .unwrap_or("examples/sorting.umlproj"),
            ),
            &p,
        )
        .expect("example save");
        return Ok(());
    }
    if args
        .get(1)
        .is_some_and(|a| a == "--export" || a == "--export-svg")
    {
        let result = (|| -> Result<(), String> {
            let p: Project = storage::load(std::path::Path::new(
                args.get(2).ok_or("프로젝트 경로 필요")?,
            ))?;
            let dir = std::path::Path::new(args.get(3).ok_or("출력 폴더 필요")?);
            std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            for (i, d) in p.diagrams.iter().enumerate() {
                let drawing = scene::build(d);
                let options = scene::ExportOptions::default();
                if args[1] == "--export-svg" {
                    uml_studio::export::svg(
                        &drawing,
                        &dir.join(format!("diagram-{}.svg", i + 1)),
                        &options,
                    )?;
                } else {
                    scene::export_png(
                        &drawing,
                        &dir.join(format!("diagram-{}.png", i + 1)),
                        &options,
                    )?;
                }
            }
            Ok(())
        })();
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return Ok(());
    }
    eframe::run_native(
        "UML Studio",
        eframe::NativeOptions {
            viewport: eframe::egui::ViewportBuilder::default()
                .with_inner_size([1440., 900.])
                .with_min_inner_size([760., 500.]),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(UmlApp::new(cc)))),
    )
}
