//! Reproducible CPU benchmark; native GPU, IME and filesystem durability are separate checks.
use eframe::{App, egui};
use std::{collections::HashSet, hint::black_box, time::Instant};
use uml_studio::{app::UmlApp, commands::History, model::*, routing, scene, storage};
fn timed<T>(f: impl FnOnce() -> T) -> (T, f64) {
    let start = Instant::now();
    let result = f();
    (result, start.elapsed().as_secs_f64() * 1000.)
}
fn fixture(nodes: usize, edges: usize) -> Project {
    let mut p = Project::new("성능 측정", DiagramKind::Class, "Grid");
    let cols = (nodes as f32).sqrt().ceil() as usize;
    let d = &mut p.diagrams[0];
    for i in 0..nodes {
        let mut n = Node::new(
            NodeKind::Class,
            Point::new((i % cols) as f32 * 205., (i / cols) as f32 * 155.),
        );
        n.name = format!("클래스 {i}");
        n.reflow();
        d.nodes.push(n);
    }
    for i in 0..edges {
        let source = i % nodes;
        let target = if i < nodes {
            if source % cols + 1 < cols && source + 1 < nodes {
                source + 1
            } else {
                source - 1
            }
        } else if source + cols < nodes {
            source + cols
        } else {
            source - cols
        };
        d.relations.push(Relation::new(
            d.nodes[source].id,
            d.nodes[target].id,
            RelationKind::Association,
        ));
    }
    p
}
fn main() {
    let out = std::path::PathBuf::from(std::env::args().nth(1).expect("output directory"));
    std::fs::create_dir_all(&out).unwrap();
    println!(
        "size,nodes,edges,load_ms,layout_ms,frame_ms,drag_ms,zoom_ms,routes_ms,hit_ms,undo_redo_ms,save_ms,png_ms,svg_ms,rss_kib"
    );
    for (size, nodes, edges) in [
        ("Small", 20, 20),
        ("Medium", 100, 150),
        ("Large", 500, 750),
        ("Stress", 1000, 1500),
    ] {
        let mut p = fixture(nodes, edges);
        let path = out.join(format!("{size}.umlproj"));
        let (_, save) = timed(|| storage::save(&path, &p).unwrap());
        let (_, load) = timed(|| storage::load(&path).unwrap());
        let (drawing, layout) = timed(|| scene::build(&p.diagrams[0]));
        let (_, routes) = timed(|| {
            for r in &p.diagrams[0].relations {
                black_box(routing::path(&p.diagrams[0], r));
            }
        });
        let (_, hit) = timed(|| {
            for i in 0..100 {
                black_box(drawing.hit(Point::new(i as f32 * 30., 100.), 6.));
            }
        });
        let before = p.clone();
        uml_studio::commands::move_nodes(
            &mut p.diagrams[0],
            &HashSet::from([before.diagrams[0].nodes[0].id]),
            Point::new(3., 2.),
        );
        let mut history = History::default();
        history.record(before, &p);
        let (_, undo) = timed(|| {
            for _ in 0..1000 {
                history.undo(&mut p);
                black_box(&p);
                history.redo(&mut p);
                black_box(&p);
            }
        });
        let ctx = egui::Context::default();
        let mut app = UmlApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.project = Some(p);
        let mut frame = eframe::Frame::_new_kittest();
        let mut tick = 0;
        let mut run = |app: &mut UmlApp| {
            tick += 1;
            let output = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440., 900.),
                    )),
                    time: Some(tick as f64 / 60.),
                    ..Default::default()
                },
                |ctx| app.update(ctx, &mut frame),
            );
            black_box(ctx.tessellate(output.shapes, 1.));
        };
        for _ in 0..3 {
            run(&mut app);
        }
        let (_, render) = timed(|| {
            for _ in 0..5 {
                run(&mut app);
            }
        });
        let (_, drag) = timed(|| {
            for _ in 0..5 {
                app.project.as_mut().unwrap().diagrams[0].nodes[0]
                    .layout
                    .pos
                    .x += 1.;
                run(&mut app);
            }
        });
        let (_, zoom) = timed(|| {
            for _ in 0..5 {
                app.project.as_mut().unwrap().diagrams[0].view.zoom += 0.05;
                run(&mut app);
            }
        });
        let options = scene::ExportOptions {
            scale: 1,
            ..Default::default()
        };
        let (_, png) = timed(|| {
            scene::export_png(&drawing, &out.join(format!("{size}.png")), &options).unwrap()
        });
        let (_, svg) = timed(|| {
            uml_studio::export::svg(&drawing, &out.join(format!("{size}.svg")), &options).unwrap()
        });
        let rss = std::process::Command::new("ps")
            .args(["-o", "rss=", "-p", &std::process::id().to_string()])
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
            .unwrap_or_else(|| "unavailable".into());
        println!(
            "{size},{nodes},{edges},{load:.3},{layout:.3},{:.3},{:.3},{:.3},{routes:.3},{:.3},{:.6},{save:.3},{png:.3},{svg:.3},{rss}",
            render / 5.,
            drag / 5.,
            zoom / 5.,
            hit / 100.,
            undo / 1000.
        );
    }
}
