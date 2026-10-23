use crate::model::*;
use eframe::egui::{self, Color32, Pos2, Rect, Stroke};
use std::{collections::HashMap, fmt::Write, path::Path};
pub const FONT: &[u8] = include_bytes!("../assets/NanumGothic.ttf");
#[derive(Clone)]
pub enum Primitive {
    Rect {
        rect: Layout,
        fill: bool,
    },
    Ellipse(Layout),
    Path {
        points: Vec<Point>,
        dash: bool,
        closed: bool,
        fill: u8,
    },
    Text {
        pos: Point,
        text: String,
        size: f32,
        italic: bool,
        underline: bool,
    },
}
#[derive(Clone)]
pub struct Item {
    pub owner: Id,
    pub primitive: Primitive,
}
#[derive(Default)]
pub struct Scene {
    pub items: Vec<Item>,
    pub boxes: HashMap<Id, Layout>,
    pub event_y: HashMap<Id, f32>,
}
impl Scene {
    fn push(&mut self, owner: Id, p: Primitive) {
        self.items.push(Item {
            owner,
            primitive: p,
        });
    }
    fn rect(&mut self, id: Id, r: Layout, fill: bool) {
        self.push(id, Primitive::Rect { rect: r, fill });
    }
    fn line(&mut self, id: Id, pts: Vec<Point>, dash: bool) {
        self.push(
            id,
            Primitive::Path {
                points: pts,
                dash,
                closed: false,
                fill: 0,
            },
        );
    }
    fn text(&mut self, id: Id, x: f32, y: f32, s: impl Into<String>, size: f32) {
        self.push(
            id,
            Primitive::Text {
                pos: Point::new(x, y),
                text: s.into(),
                size,
                italic: false,
                underline: false,
            },
        );
    }
    pub fn bounds(&self) -> Option<Layout> {
        let mut min = Point::new(f32::INFINITY, f32::INFINITY);
        let mut max = Point::new(f32::NEG_INFINITY, f32::NEG_INFINITY);
        for item in &self.items {
            let r = primitive_bounds(&item.primitive);
            min.x = min.x.min(r.pos.x);
            min.y = min.y.min(r.pos.y);
            max.x = max.x.max(r.pos.x + r.size.x);
            max.y = max.y.max(r.pos.y + r.size.y);
        }
        if !min.x.is_finite() {
            None
        } else {
            Some(Layout::new(
                min.x - 3.,
                min.y - 3.,
                max.x - min.x + 6.,
                max.y - min.y + 6.,
            ))
        }
    }
    pub fn selection_boxes(&self) -> HashMap<Id, Layout> {
        let mut boxes = self.boxes.clone();
        for item in &self.items {
            if self.boxes.contains_key(&item.owner) {
                continue;
            }
            let r = primitive_bounds(&item.primitive);
            boxes
                .entry(item.owner)
                .and_modify(|b| {
                    let x = b.pos.x.min(r.pos.x);
                    let y = b.pos.y.min(r.pos.y);
                    let right = (b.pos.x + b.size.x).max(r.pos.x + r.size.x);
                    let bottom = (b.pos.y + b.size.y).max(r.pos.y + r.size.y);
                    *b = Layout::new(x, y, right - x, bottom - y);
                })
                .or_insert(r);
        }
        boxes
    }
    pub fn hit(&self, p: Point, tolerance: f32) -> Option<Id> {
        self.hits(p, tolerance).first().copied()
    }
    pub fn hits(&self, p: Point, tolerance: f32) -> Vec<Id> {
        let mut out = vec![];
        for item in self.items.iter().rev() {
            let yes = match &item.primitive {
                Primitive::Path { points, .. } => points
                    .windows(2)
                    .any(|w| segment_distance(p, w[0], w[1]) < tolerance),
                Primitive::Rect { rect, fill: false } => {
                    let l = *rect;
                    let a = l.pos;
                    let b = Point::new(a.x + l.size.x, a.y);
                    let c = Point::new(b.x, a.y + l.size.y);
                    let d = Point::new(a.x, c.y);
                    [(a, b), (b, c), (c, d), (d, a)]
                        .iter()
                        .any(|(a, b)| segment_distance(p, *a, *b) < tolerance)
                        || Layout::new(a.x, a.y, l.size.x, 28.).contains(p)
                }
                _ => primitive_bounds(&item.primitive).contains(p),
            };
            if yes && !out.contains(&item.owner) {
                out.push(item.owner);
            }
        }
        out
    }
}
pub fn segment_distance(p: Point, a: Point, b: Point) -> f32 {
    let dx = b.x - a.x;
    let dy = b.y - a.y;
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy).max(0.0001)).clamp(0., 1.);
    ((p.x - a.x - t * dx).powi(2) + (p.y - a.y - t * dy).powi(2)).sqrt()
}
fn primitive_bounds(p: &Primitive) -> Layout {
    match p {
        Primitive::Rect { rect, .. } | Primitive::Ellipse(rect) => *rect,
        Primitive::Text {
            pos,
            text,
            size,
            italic,
            ..
        } => Layout::new(
            pos.x - 2.,
            pos.y - 2.,
            text_width(text, *size) + 4. + if *italic { size * 0.3 } else { 0. },
            size * 1.7,
        ),
        Primitive::Path { points, .. } => {
            let x = points.iter().map(|p| p.x).fold(f32::INFINITY, f32::min);
            let y = points.iter().map(|p| p.y).fold(f32::INFINITY, f32::min);
            let r = points.iter().map(|p| p.x).fold(f32::NEG_INFINITY, f32::max);
            let b = points.iter().map(|p| p.y).fold(f32::NEG_INFINITY, f32::max);
            Layout::new(x - 2., y - 2., r - x + 4., b - y + 4.)
        }
    }
}
fn arrow(s: &mut Scene, id: Id, end: Point, previous: Point, kind: u8) {
    let dx = end.x - previous.x;
    let dy = end.y - previous.y;
    let len = dx.hypot(dy).max(0.01);
    let ux = dx / len;
    let uy = dy / len;
    let back = Point::new(end.x - ux * 13., end.y - uy * 13.);
    let a = Point::new(back.x - uy * 6., back.y + ux * 6.);
    let b = Point::new(back.x + uy * 6., back.y - ux * 6.);
    let points = if kind >= 3 {
        vec![end, a, Point::new(end.x - ux * 26., end.y - uy * 26.), b]
    } else {
        vec![a, end, b]
    };
    s.push(
        id,
        Primitive::Path {
            points,
            dash: false,
            closed: kind != 0,
            fill: if kind == 2 || kind == 4 {
                2
            } else if kind == 0 {
                0
            } else {
                1
            },
        },
    );
}
fn actor(s: &mut Scene, id: Id, r: Layout, name: &str) {
    let (body, lines) = crate::routing::actor_body(r, name);
    let map = |x: f32, y: f32| {
        Point::new(
            body.pos.x + x / 90. * body.size.x,
            body.pos.y + y / 100. * body.size.y,
        )
    };
    let h = map(33., 3.);
    s.push(
        id,
        Primitive::Ellipse(Layout::new(
            h.x,
            h.y,
            body.size.x * 24. / 90.,
            body.size.y * 24. / 100.,
        )),
    );
    for coords in [
        vec![(45., 27.), (45., 70.)],
        vec![(20., 43.), (70., 43.)],
        vec![(22., 96.), (45., 70.), (68., 96.)],
    ] {
        s.line(
            id,
            coords.into_iter().map(|(x, y)| map(x, y)).collect(),
            false,
        );
    }
    for (i, line) in lines.iter().enumerate() {
        s.text(
            id,
            r.center().x - text_width(line, 14.) / 2.,
            r.pos.y + body.size.y + 4. + i as f32 * 22.,
            line,
            14.,
        );
    }
}
pub fn build(d: &Diagram) -> Scene {
    let mut prepared = d.clone();
    for n in &mut prepared.nodes {
        n.reflow();
    }
    let d = &prepared;
    if d.kind == DiagramKind::Sequence {
        return sequence_scene(&d.sequence);
    }
    let mut s = Scene::default();
    for n in d
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Boundary | NodeKind::Package))
    {
        s.boxes.insert(n.id, n.layout);
        s.rect(n.id, n.layout, false);
        s.text(
            n.id,
            n.layout.pos.x + 10.,
            n.layout.pos.y + 5.,
            &n.name,
            16.,
        );
    }
    let mut relation_labels = Vec::new();
    // Relations are rendered behind nodes; endpoints are attached to current geometry.
    for r in &d.relations {
        let pts = crate::routing::path(d, r);
        if pts.len() < 2 {
            continue;
        }
        let dash = matches!(
            r.kind,
            RelationKind::Dependency
                | RelationKind::Realization
                | RelationKind::Include
                | RelationKind::Extend
                | RelationKind::Comment
        );
        s.line(r.id, pts.clone(), dash);
        let end = *pts.last().unwrap();
        let start = pts[0];
        match r.kind {
            RelationKind::Generalization | RelationKind::Realization => {
                arrow(&mut s, r.id, end, pts[pts.len() - 2], 1)
            }
            RelationKind::Aggregation | RelationKind::Composition => arrow(
                &mut s,
                r.id,
                start,
                pts[1],
                if r.kind == RelationKind::Composition {
                    4
                } else {
                    3
                },
            ),
            RelationKind::Comment => {}
            RelationKind::Association => {
                if r.navigable {
                    arrow(&mut s, r.id, end, pts[pts.len() - 2], 0)
                }
            }
            _ => arrow(&mut s, r.id, end, pts[pts.len() - 2], 0),
        }
        let label = match r.kind {
            RelationKind::Include => format!("«include» {}", r.name),
            RelationKind::Extend => format!("«extend» {}", r.name),
            _ => r.name.clone(),
        };
        if !label.trim().is_empty() {
            let lines = crate::text_layout::wrap(&label, 13., 220.);
            let size = Point::new(
                lines.iter().map(|s| text_width(s, 13.)).fold(0., f32::max),
                lines.len() as f32 * 22.,
            );
            let origin = crate::routing::label_position(d, &pts, size);
            for (i, line) in lines.into_iter().enumerate() {
                relation_labels.push((
                    r.id,
                    Point::new(origin.x, origin.y + i as f32 * 22.),
                    line,
                    13.,
                ));
            }
        }

        for (endpoint, toward, text) in [
            (
                start,
                pts[1],
                format!("{} {}", r.source_role, r.source_multiplicity),
            ),
            (
                end,
                pts[pts.len() - 2],
                format!("{} {}", r.target_role, r.target_multiplicity),
            ),
        ] {
            if text.trim().is_empty() {
                continue;
            }
            let dx = toward.x - endpoint.x;
            let dy = toward.y - endpoint.y;
            let pos = if dx.abs() >= dy.abs() {
                Point::new(
                    if dx >= 0. {
                        endpoint.x + 9.
                    } else {
                        endpoint.x - text_width(&text, 12.) - 9.
                    },
                    endpoint.y + 8.,
                )
            } else {
                Point::new(
                    endpoint.x + 10.,
                    if dy >= 0. {
                        endpoint.y + 8.
                    } else {
                        endpoint.y - 26.
                    },
                )
            };
            relation_labels.push((r.id, pos, text, 12.));
        }
    }
    for n in d
        .nodes
        .iter()
        .filter(|n| !matches!(n.kind, NodeKind::Boundary | NodeKind::Package))
    {
        let r = n.layout;
        let x = r.pos.x;
        let y = r.pos.y;
        s.boxes.insert(n.id, r);
        match n.kind {
            NodeKind::Actor => actor(&mut s, n.id, r, &n.name),
            NodeKind::UseCase => {
                s.push(n.id, Primitive::Ellipse(r));
                let lines = crate::text_layout::wrap(&n.name, 15., r.size.x * 0.72);
                for (i, line) in lines.iter().enumerate() {
                    s.text(
                        n.id,
                        x + (r.size.x - text_width(line, 15.)) / 2.,
                        y + (r.size.y - lines.len() as f32 * 23.) / 2. + i as f32 * 23.,
                        line,
                        15.,
                    );
                }
            }
            NodeKind::Note => {
                s.rect(n.id, r, true);
                s.line(
                    n.id,
                    vec![
                        Point::new(x + r.size.x - 18., y),
                        Point::new(x + r.size.x - 18., y + 18.),
                        Point::new(x + r.size.x, y + 18.),
                    ],
                    false,
                );
                for (i, line) in crate::text_layout::wrap(&n.name, 14., r.size.x - 28.)
                    .iter()
                    .enumerate()
                {
                    s.text(n.id, x + 10., y + 25. + i as f32 * 23., line, 14.);
                }
            }
            _ => {
                let layout = crate::text_layout::class_layout(
                    n,
                    if n.sizing == SizeMode::Manual {
                        Some(r.size.x)
                    } else {
                        None
                    },
                );
                s.rect(n.id, r, true);
                for yy in layout.separators {
                    s.line(
                        n.id,
                        vec![Point::new(x, y + yy), Point::new(x + r.size.x, y + yy)],
                        false,
                    );
                }
                for line in layout.lines {
                    s.push(
                        n.id,
                        Primitive::Text {
                            pos: Point::new(x + line.pos.x, y + line.pos.y),
                            text: line.text,
                            size: line.size,
                            italic: line.italic,
                            underline: line.underline,
                        },
                    );
                }
            }
        }
    }
    for (id, pos, text, size) in relation_labels {
        s.text(id, pos.x, pos.y, text, size);
    }

    s
}
fn sequence_scene(seq: &Sequence) -> Scene {
    let layout = crate::sequence_layout::layout(seq);
    let mut s = Scene {
        event_y: layout.events.clone(),
        ..Default::default()
    };
    for f in &layout.fragments {
        s.rect(f.id, f.rect, false);
        s.boxes.insert(f.id, f.rect);
        for (i, line) in f.title.iter().enumerate() {
            s.text(
                f.id,
                f.rect.pos.x + 10.,
                f.rect.pos.y + 6. + i as f32 * 23.,
                line,
                14.,
            );
        }
        if let Some((y, lines)) = &f.split {
            s.line(
                f.id,
                vec![
                    Point::new(f.rect.pos.x, *y),
                    Point::new(f.rect.pos.x + f.rect.size.x, *y),
                ],
                true,
            );
            for (i, line) in lines.iter().enumerate() {
                s.text(f.id, f.rect.pos.x + 12., y + 5. + i as f32 * 22., line, 13.);
            }
        }
    }
    for p in &seq.participants {
        let r = layout.participants[&p.id];
        s.boxes.insert(p.id, r);
        if p.actor {
            actor(&mut s, p.id, r, &p.name);
        } else {
            s.rect(p.id, r, true);
            let lines = crate::text_layout::wrap(&p.name, 15., r.size.x - 20.);
            for (i, line) in lines.iter().enumerate() {
                s.text(
                    p.id,
                    r.center().x - text_width(line, 15.) / 2.,
                    r.pos.y + 12. + i as f32 * 23.,
                    line,
                    15.,
                );
            }
        }
        let (start, end) = layout.lifelines[&p.id];
        s.line(
            p.id,
            vec![
                Point::new(r.center().x, start),
                Point::new(r.center().x, end),
            ],
            true,
        );
    }
    let mut bars: Vec<_> = layout.activations.iter().collect();
    bars.sort_by(|a, b| a.1.pos.x.total_cmp(&b.1.pos.x));
    for (&id, &rect) in bars {
        s.rect(id, rect, true);
        s.boxes.insert(id, rect);
    }
    for m in &layout.messages {
        s.line(
            m.id,
            m.points.clone(),
            matches!(m.kind, MessageKind::Return | MessageKind::Create),
        );
        arrow(
            &mut s,
            m.id,
            *m.points.last().unwrap(),
            m.points[m.points.len() - 2],
            if m.kind == MessageKind::Sync { 2 } else { 0 },
        );
        for (pos, line) in &m.labels {
            s.text(m.id, pos.x, pos.y, line, 14.);
        }
    }
    for (&id, &point) in &layout.destroyed {
        for sign in [-1., 1.] {
            s.line(
                id,
                vec![
                    Point::new(point.x - 10., point.y - 10. * sign),
                    Point::new(point.x + 10., point.y + 10. * sign),
                ],
                false,
            );
        }
    }
    s
}
fn color() -> Color32 {
    Color32::from_rgb(36, 48, 68)
}
pub fn paint(scene: &Scene, painter: &egui::Painter, origin: Pos2, pan: Point, zoom: f32) {
    let map = |p: Point| Pos2::new(origin.x + pan.x + p.x * zoom, origin.y + pan.y + p.y * zoom);
    let stroke = Stroke::new(1.5 * zoom, color());
    for item in &scene.items {
        match &item.primitive {
            Primitive::Rect { rect, fill } => {
                let r = Rect::from_min_size(
                    map(rect.pos),
                    egui::vec2(rect.size.x * zoom, rect.size.y * zoom),
                );
                painter.rect(
                    r,
                    0.,
                    if *fill {
                        Color32::WHITE
                    } else {
                        Color32::TRANSPARENT
                    },
                    stroke,
                    egui::StrokeKind::Inside,
                );
            }
            Primitive::Ellipse(r) => {
                painter.add(egui::epaint::EllipseShape {
                    center: map(r.center()),
                    radius: egui::vec2(r.size.x * zoom / 2., r.size.y * zoom / 2.),
                    fill: Color32::WHITE,
                    stroke,
                });
            }
            Primitive::Path {
                points,
                dash,
                closed,
                fill,
            } => {
                let p: Vec<_> = points.iter().map(|p| map(*p)).collect();
                if *closed {
                    painter.add(egui::Shape::convex_polygon(
                        p,
                        if *fill == 2 { color() } else { Color32::WHITE },
                        stroke,
                    ));
                } else {
                    for w in p.windows(2) {
                        if *dash {
                            let v = w[1] - w[0];
                            let len = v.length();
                            let dir = v.normalized();
                            let mut at = 0.;
                            while at < len {
                                painter.line_segment(
                                    [w[0] + dir * at, w[0] + dir * (at + 6. * zoom).min(len)],
                                    stroke,
                                );
                                at += 11. * zoom;
                            }
                        } else {
                            painter.line_segment([w[0], w[1]], stroke);
                        }
                    }
                }
            }
            Primitive::Text {
                pos,
                text,
                size,
                italic,
                underline,
            } => {
                let mut job = egui::text::LayoutJob::default();
                job.append(
                    text,
                    0.,
                    egui::TextFormat {
                        font_id: egui::FontId::proportional(size * zoom),
                        color: color(),
                        italics: *italic,
                        underline: if *underline {
                            Stroke::new(zoom, color())
                        } else {
                            Stroke::NONE
                        },
                        ..Default::default()
                    },
                );
                let galley = painter.layout_job(job);
                painter.galley(map(*pos), galley, color());
            }
        }
    }
}
fn escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[derive(Clone)]
pub struct ExportOptions {
    pub scale: u32,
    pub margin: u32,
    pub transparent: bool,
}
impl Default for ExportOptions {
    fn default() -> Self {
        Self {
            scale: 2,
            margin: 32,
            transparent: false,
        }
    }
}
pub fn svg(scene: &Scene, options: &ExportOptions) -> Result<(String, u32, u32), String> {
    let r = scene.bounds().ok_or("빈 다이어그램은 내보낼 수 없습니다")?;
    if !(1..=3).contains(&options.scale) || options.margin > 1000 {
        return Err("배율 또는 여백이 올바르지 않습니다".into());
    }
    let w = (r.size.x.ceil() + options.margin as f32 * 2.) * options.scale as f32;
    let h = (r.size.y.ceil() + options.margin as f32 * 2.) * options.scale as f32;
    if !w.is_finite() || !h.is_finite() || w > 16384. || h > 16384. || w * h > 40_000_000. {
        return Err("PNG 최대 크기(16,384px / 4천만 픽셀)를 초과했습니다. 배율을 낮추세요".into());
    }
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}\" height=\"{h}\" viewBox=\"{} {} {} {}\">",
        r.pos.x - options.margin as f32,
        r.pos.y - options.margin as f32,
        w / options.scale as f32,
        h / options.scale as f32
    );
    if !options.transparent {
        let _ = write!(
            out,
            "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"white\"/>",
            r.pos.x - options.margin as f32,
            r.pos.y - options.margin as f32,
            w / options.scale as f32,
            h / options.scale as f32
        );
    }
    out.push_str(
        "<g stroke=\"#243044\" stroke-width=\"1.5\" fill=\"none\" stroke-linejoin=\"round\">",
    );
    for item in &scene.items {
        match &item.primitive {
            Primitive::Rect { rect: r, fill } => {
                let _ = write!(
                    out,
                    "<rect x=\"{}\" y=\"{}\" width=\"{}\" height=\"{}\" fill=\"{}\"/>",
                    r.pos.x,
                    r.pos.y,
                    r.size.x,
                    r.size.y,
                    if *fill { "white" } else { "none" }
                );
            }
            Primitive::Ellipse(r) => {
                let c = r.center();
                let _ = write!(
                    out,
                    "<ellipse cx=\"{}\" cy=\"{}\" rx=\"{}\" ry=\"{}\" fill=\"white\"/>",
                    c.x,
                    c.y,
                    r.size.x / 2.,
                    r.size.y / 2.
                );
            }
            Primitive::Path {
                points,
                dash,
                closed,
                fill,
            } => {
                let pts = points
                    .iter()
                    .map(|p| format!("{},{}", p.x, p.y))
                    .collect::<Vec<_>>()
                    .join(" ");
                let _ = write!(
                    out,
                    "<{} points=\"{}\" fill=\"{}\" {}/>",
                    if *closed { "polygon" } else { "polyline" },
                    pts,
                    match fill {
                        1 => "white",
                        2 => "#243044",
                        _ => "none",
                    },
                    if *dash {
                        "stroke-dasharray=\"6 5\""
                    } else {
                        ""
                    }
                );
            }
            Primitive::Text {
                pos,
                text,
                size,
                italic,
                underline,
            } => {
                let baseline = pos.y + size * 1.05;
                // The bundled font has no italic face; apply the same synthetic slant as the UI.
                let transform = if *italic {
                    format!("translate({} {}) skewX(-12)", pos.x, baseline)
                } else {
                    format!("translate({} {})", pos.x, baseline)
                };
                let _ = write!(
                    out,
                    "<text x=\"0\" y=\"0\" transform=\"{}\" font-family=\"NanumGothic\" font-size=\"{}\" fill=\"#243044\" stroke=\"none\" text-decoration=\"{}\">{}</text>",
                    transform,
                    size,
                    if *underline { "underline" } else { "none" },
                    escape(text)
                );
            }
        }
    }
    out.push_str("</g></svg>");
    Ok((out, w as u32, h as u32))
}
pub fn export_png(
    scene: &Scene,
    path: &Path,
    options: &ExportOptions,
) -> Result<(u32, u32), String> {
    crate::export::png(scene, path, options)
}
