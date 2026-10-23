//! Headless egui layout snapshot. No native window, OS events, or screenshot API.
//! A small CPU renderer for egui meshes, intended for review artifacts only.
use eframe::{
    App,
    egui::{self, Color32, Pos2},
};
use std::collections::HashMap;
struct Texture {
    width: usize,
    height: usize,
    pixels: Vec<Color32>,
}
fn cross(a: Pos2, b: Pos2, c: Pos2) -> f32 {
    (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x)
}
fn text_center(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
    match shape {
        egui::Shape::Text(t) if t.galley.job.text == label => Some(t.pos + t.galley.size() / 2.),
        egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| text_center(s, label)),
        _ => None,
    }
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    let path = args.get(1).expect("output PNG path");
    let index: usize = args.get(2).and_then(|a| a.parse().ok()).unwrap_or(0);
    let small = args.iter().any(|a| a == "--small");
    let (width, height) = if small { (820, 620) } else { (1440, 900) };
    let ctx = egui::Context::default();
    let mut app = uml_studio::app::UmlApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
    let mut p = if args.iter().any(|a| a == "--relations") {
        uml_studio::demo::relationship_review_project()
    } else {
        uml_studio::demo::review_project()
    };
    let len = p.diagrams.len();
    p.diagrams.rotate_left(index % len);
    let selected = if args.iter().any(|a| a == "--relations") {
        p.diagrams[0].relations.get(3).map(|r| r.id)
    } else if args.iter().any(|a| a == "--relation-inspector") {
        p.diagrams[0].relations.first().map(|r| r.id)
    } else {
        match index % 3 {
            0 => p.diagrams[0]
                .nodes
                .iter()
                .find(|n| n.kind == uml_studio::model::NodeKind::UseCase)
                .map(|n| n.id),
            1 => p.diagrams[0]
                .nodes
                .iter()
                .find(|n| n.is_classifier())
                .map(|n| n.id),
            _ => p.diagrams[0].sequence.fragments.first().map(|f| f.id),
        }
    };
    if args.iter().any(|a| a == "--long-names") {
        for d in &mut p.diagrams {
            d.name = format!("{} · 긴 다이어그램 이름과 상세 설계 검토", d.name);
        }
    }
    let tab_elements: Vec<_> = p
        .diagrams
        .iter()
        .filter_map(|d| {
            d.nodes
                .first()
                .map(|n| n.id)
                .or_else(|| d.sequence.participants.first().map(|n| n.id))
        })
        .collect();
    app.project = Some(p);
    for id in tab_elements {
        app.inspect_element(id);
    }
    if let Some(id) = selected {
        app.inspect_element(id);
    }
    let mut frame = eframe::Frame::_new_kittest();
    let mut textures = HashMap::new();
    let mut shapes = vec![];
    let mut menu_pos = None;
    let ticks = if args
        .iter()
        .any(|a| a == "--route-settings" || a == "--relation-menu")
    {
        30
    } else {
        12
    };
    for tick in 0..ticks {
        if tick == 2
            && args.iter().any(|a| a == "--inspector")
            && let Some(id) = selected
        {
            app.inspect_element(id);
        }
        let mut events = vec![];
        if args.iter().any(|a| a == "--relation-menu") {
            if tick == 6 {
                menu_pos = shapes
                    .iter()
                    .find_map(|s: &egui::epaint::ClippedShape| text_center(&s.shape, "관계 연결"));
            }
            if let Some(pos) = menu_pos {
                if tick == 7 {
                    events.push(egui::Event::PointerMoved(pos));
                }
                if tick == 8 || tick == 9 {
                    events.push(egui::Event::PointerButton {
                        pos,
                        button: egui::PointerButton::Primary,
                        pressed: tick == 8,
                        modifiers: egui::Modifiers::NONE,
                    });
                }
            }
        }
        if tick == 7 && args.iter().any(|a| a == "--route-settings") {
            events.push(egui::Event::PointerMoved(egui::pos2(
                width as f32 - 80.,
                height as f32 - 160.,
            )));
            events.push(egui::Event::MouseWheel {
                unit: egui::MouseWheelUnit::Point,
                delta: egui::vec2(0., -620.),
                modifiers: egui::Modifiers::NONE,
            });
        }
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    Pos2::ZERO,
                    egui::vec2(width as f32, height as f32),
                )),
                events,
                time: Some(tick as f64 / 30.),
                ..Default::default()
            },
            |ctx| app.update(ctx, &mut frame),
        );
        if tick == 4 {
            app.fit_active();
        }
        for (id, delta) in output.textures_delta.set {
            let egui::ImageData::Color(image) = delta.image;
            if let Some([x, y]) = delta.pos {
                let texture: &mut Texture =
                    textures.get_mut(&id).expect("full texture before patch");
                for row in 0..image.height() {
                    texture.pixels[(y + row) * texture.width + x
                        ..(y + row) * texture.width + x + image.width()]
                        .copy_from_slice(
                            &image.pixels[row * image.width()..(row + 1) * image.width()],
                        );
                }
            } else {
                textures.insert(
                    id,
                    Texture {
                        width: image.width(),
                        height: image.height(),
                        pixels: image.pixels.clone(),
                    },
                );
            }
        }
        shapes = output.shapes;
    }
    let primitives = ctx.tessellate(shapes, 1.);
    let mut data = vec![255_u8; width * height * 4];
    for primitive in primitives {
        let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive else {
            panic!("custom callback not supported in CPU snapshot")
        };
        let texture = textures.get(&mesh.texture_id);
        let clip = primitive.clip_rect;
        for triangle in mesh.indices.as_chunks::<3>().0 {
            let v = [
                mesh.vertices[triangle[0] as usize],
                mesh.vertices[triangle[1] as usize],
                mesh.vertices[triangle[2] as usize],
            ];
            let area = cross(v[0].pos, v[1].pos, v[2].pos);
            if area.abs() < 0.0001 {
                continue;
            }
            let min_x = v
                .iter()
                .map(|p| p.pos.x)
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(clip.min.x)
                .max(0.) as usize;
            let min_y = v
                .iter()
                .map(|p| p.pos.y)
                .fold(f32::INFINITY, f32::min)
                .floor()
                .max(clip.min.y)
                .max(0.) as usize;
            let max_x = v
                .iter()
                .map(|p| p.pos.x)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .min(clip.max.x)
                .min(width as f32) as usize;
            let max_y = v
                .iter()
                .map(|p| p.pos.y)
                .fold(f32::NEG_INFINITY, f32::max)
                .ceil()
                .min(clip.max.y)
                .min(height as f32) as usize;
            for y in min_y..max_y {
                for x in min_x..max_x {
                    let point = egui::pos2(x as f32 + 0.5, y as f32 + 0.5);
                    let bary = [
                        cross(v[1].pos, v[2].pos, point) / area,
                        cross(v[2].pos, v[0].pos, point) / area,
                        cross(v[0].pos, v[1].pos, point) / area,
                    ];
                    if bary.iter().any(|b| *b < -0.0001) {
                        continue;
                    }
                    let uv = egui::pos2(
                        v.iter().zip(bary).map(|(v, b)| v.uv.x * b).sum(),
                        v.iter().zip(bary).map(|(v, b)| v.uv.y * b).sum(),
                    );
                    let texel = texture
                        .map(|t| {
                            let tx = (uv.x * t.width as f32)
                                .floor()
                                .clamp(0., t.width as f32 - 1.)
                                as usize;
                            let ty = (uv.y * t.height as f32)
                                .floor()
                                .clamp(0., t.height as f32 - 1.)
                                as usize;
                            t.pixels[ty * t.width + tx].to_array()
                        })
                        .unwrap_or([255; 4]);
                    let color: [f32; 4] = std::array::from_fn(|channel| {
                        v.iter()
                            .zip(bary)
                            .map(|(v, b)| v.color.to_array()[channel] as f32 * b)
                            .sum::<f32>()
                            / 255.
                            * texel[channel] as f32
                            / 255.
                    });
                    let offset = (y * width + x) * 4;
                    for c in 0..4 {
                        data[offset + c] =
                            ((color[c] + data[offset + c] as f32 / 255. * (1. - color[3])) * 255.)
                                .clamp(0., 255.) as u8;
                    }
                }
            }
        }
    }
    let pixels = resvg::tiny_skia::Pixmap::from_vec(
        data,
        resvg::tiny_skia::IntSize::from_wh(width as u32, height as u32).unwrap(),
    )
    .unwrap();
    pixels.save_png(path).unwrap();
    println!(
        "Headless egui snapshot: {path} ({width}x{height}), {} textures",
        textures.len()
    );
}
