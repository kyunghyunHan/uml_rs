use crate::{commands, model::*, scene};
use eframe::egui::{self, Color32, Key, PointerButton, Rect, Sense, Stroke};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Add(NodeKind),
    Connect(RelationKind),
}
#[derive(Clone, Copy)]
pub enum ContextAction {
    Delete,
    Copy,
    Paste,
    Duplicate,
}
#[derive(Default)]
pub struct CanvasState {
    pub selection: HashSet<Id>,
    pub grid: bool,
    pub snap: bool,
    pub connection: Option<Id>,
    connection_drag: Option<Id>,
    connection_kind: Option<RelationKind>,
    drag: Option<Drag>,
    pub finished_drag: bool,
    pub edit_name: Option<Id>,
    pub notice: Option<String>,
    pub context_action: Option<ContextAction>,
    pub guides: bool,
}
struct Drag {
    start: Point,
    last: Point,
    original: HashMap<Id, Layout>,
    marquee: bool,
    resize: Option<Id>,
    additive: bool,
    initial_selection: HashSet<Id>,
    route: Option<(Id, usize, Vec<Point>)>,
}
pub fn screen_to_world(screen: egui::Pos2, origin: egui::Pos2, view: View) -> Point {
    Point::new(
        (screen.x - origin.x - view.pan.x) / view.zoom,
        (screen.y - origin.y - view.pan.y) / view.zoom,
    )
}
pub fn world_to_screen(p: Point, origin: egui::Pos2, view: View) -> egui::Pos2 {
    origin + egui::vec2(view.pan.x + p.x * view.zoom, view.pan.y + p.y * view.zoom)
}
pub fn fit(d: &mut Diagram, size: egui::Vec2) {
    if let Some(r) = scene::build(d).bounds() {
        d.view.zoom = ((size.x - 100.) / r.size.x)
            .min((size.y - 100.) / r.size.y)
            .clamp(0.1, 2.);
        d.view.pan = Point::new(
            (size.x - r.size.x * d.view.zoom) / 2. - r.pos.x * d.view.zoom,
            (size.y - r.size.y * d.view.zoom) / 2. - r.pos.y * d.view.zoom,
        );
    }
}

// Connecting targets nodes, even when a relation label or an existing line is
// painted above them. Actors use their whole selectable box, not just the strokes.
fn connection_target(d: &Diagram, sc: &scene::Scene, p: Point, tolerance: f32) -> Option<Id> {
    let hits = sc.hits(p, tolerance);
    d.nodes
        .iter()
        .rev()
        .find(|n| {
            !matches!(n.kind, NodeKind::Boundary | NodeKind::Package)
                && (n.layout.contains(p) || hits.contains(&n.id))
        })
        .map(|n| n.id)
        // Container interiors remain transparent, so they cannot swallow a click.
        .or_else(|| {
            hits.into_iter()
                .find(|id| d.nodes.iter().any(|n| n.id == *id))
        })
}
impl CanvasState {
    pub fn new() -> Self {
        Self {
            grid: true,
            guides: true,
            ..Default::default()
        }
    }
    pub fn clear(&mut self) {
        self.selection.clear();
        self.connection = None;
        self.connection_drag = None;
        self.notice = None;
        self.drag = None;
    }
    fn connect_to(&mut self, d: &mut Diagram, target: Id, kind: RelationKind) -> bool {
        let Some(source) = self.connection.take() else {
            if let Err(reason) = crate::validation::connection_start(d, target, kind) {
                self.notice = Some(reason);
                self.connection_drag = None;
                return false;
            }
            self.connection = Some(target);
            self.notice = None;
            return false;
        };
        let relation = Relation::new(source, target, kind);
        let id = relation.id;
        match d.add_relation(relation) {
            Ok(()) => {
                self.selection.clear();
                self.selection.insert(id);
                self.notice = None;
                true
            }
            Err(reason) => {
                self.notice = Some(reason);
                self.connection = Some(source);
                false
            }
        }
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, d: &mut Diagram, tool: &mut Tool) -> bool {
        for n in &mut d.nodes {
            n.reflow();
        }
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        let origin = rect.min;
        let mut changed = false;
        self.finished_drag = false;
        painter.rect_filled(rect, 0., Color32::from_rgb(247, 249, 252));
        let pointer = ui.input(|i| i.pointer.hover_pos());
        let kind = match *tool {
            Tool::Connect(kind) => Some(kind),
            _ => None,
        };
        if self.connection_kind != kind {
            self.connection = None;
            self.connection_drag = None;
            self.notice = None;
            self.connection_kind = kind;
        }
        if ui.is_enabled()
            && !ui.ctx().wants_keyboard_input()
            && ui.input(|i| i.key_pressed(Key::Escape))
        {
            *tool = Tool::Select;
            self.notice = None;
        }
        if !matches!(*tool, Tool::Connect(_))
            || self
                .connection
                .is_some_and(|id| !d.nodes.iter().any(|n| n.id == id))
        {
            self.connection = None;
            self.connection_drag = None;
        }
        if response.hovered() {
            let (scroll, zoom, mods) =
                ui.input(|i| (i.smooth_scroll_delta, i.zoom_delta(), i.modifiers));
            if let Some(pos) = pointer {
                let factor = if zoom != 1. {
                    zoom
                } else if mods.command || mods.ctrl {
                    (scroll.y * 0.004).exp()
                } else {
                    1.
                };
                if factor != 1. {
                    let w = screen_to_world(pos, origin, d.view);
                    d.view.zoom = (d.view.zoom * factor).clamp(0.1, 5.);
                    d.view.pan = Point::new(
                        pos.x - origin.x - w.x * d.view.zoom,
                        pos.y - origin.y - w.y * d.view.zoom,
                    );
                } else {
                    d.view.pan.x += scroll.x;
                    d.view.pan.y += scroll.y;
                }
            }
        }
        if response.dragged_by(PointerButton::Middle)
            || response.dragged_by(PointerButton::Secondary)
        {
            let delta = ui.input(|i| i.pointer.delta());
            d.view.pan.x += delta.x;
            d.view.pan.y += delta.y;
        }
        if self.grid && d.view.zoom > 0.25 {
            let spacing = 20. * d.view.zoom;
            let mut x = rect.left() + d.view.pan.x.rem_euclid(spacing);
            while x < rect.right() {
                let mut y = rect.top() + d.view.pan.y.rem_euclid(spacing);
                while y < rect.bottom() {
                    painter.circle_filled(egui::pos2(x, y), 1., Color32::from_rgb(211, 219, 229));
                    y += spacing;
                }
                x += spacing;
            }
        }
        let sc = scene::build(d);
        let space = ui.input(|i| i.key_down(Key::Space)) && !ui.ctx().wants_keyboard_input();
        if response.dragged_by(PointerButton::Primary) && space {
            let delta = ui.input(|i| i.pointer.delta());
            d.view.pan.x += delta.x;
            d.view.pan.y += delta.y;
            self.drag = None;
        }
        if *tool == Tool::Select
            && response.double_clicked()
            && let Some(pos) = pointer
            && let Some(id) = sc.hit(screen_to_world(pos, origin, d.view), 7. / d.view.zoom)
        {
            self.edit_name = Some(id);
            self.selection.clear();
            self.selection.insert(id);
        }
        let route_handles: Vec<_> = d
            .relations
            .iter()
            .filter(|r| self.selection.contains(&r.id))
            .flat_map(|r| {
                let pts = crate::routing::path(d, r);
                let inner = if !r.waypoints.is_empty() {
                    r.waypoints.clone()
                } else if pts.len() > 2 {
                    pts[1..pts.len() - 1].to_vec()
                } else if pts.len() == 2 {
                    vec![Point::new(
                        (pts[0].x + pts[1].x) / 2.,
                        (pts[0].y + pts[1].y) / 2.,
                    )]
                } else {
                    vec![]
                };
                inner
                    .iter()
                    .enumerate()
                    .map(|(i, p)| (r.id, i, *p, inner.clone()))
                    .collect::<Vec<_>>()
            })
            .collect();
        let primary_pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        if response.hovered()
            && !space
            && primary_pressed
            && let Some(pos) = pointer
        {
            let w = screen_to_world(pos, origin, d.view);
            let candidates = sc.hits(w, 7. / d.view.zoom);
            let hit = if ui.input(|i| i.modifiers.alt) {
                let current = candidates.iter().position(|id| self.selection.contains(id));
                candidates
                    .get(current.map_or(0, |i| (i + 1) % candidates.len().max(1)))
                    .copied()
            } else {
                candidates.first().copied()
            };
            let additive = ui.input(|i| i.modifiers.shift || i.modifiers.command);
            match *tool {
                Tool::Add(kind) => {
                    let mut n = Node::new(kind, w);
                    if self.snap {
                        n.layout.pos.x = (w.x / 20.).round() * 20.;
                        n.layout.pos.y = (w.y / 20.).round() * 20.;
                    }
                    self.selection.clear();
                    self.selection.insert(n.id);
                    if !matches!(kind, NodeKind::Boundary | NodeKind::Package) {
                        n.boundary = d
                            .nodes
                            .iter()
                            .filter(|b| {
                                matches!(b.kind, NodeKind::Boundary | NodeKind::Package)
                                    && b.layout.contains(n.layout.center())
                            })
                            .min_by(|a, b| {
                                (a.layout.size.x * a.layout.size.y)
                                    .total_cmp(&(b.layout.size.x * b.layout.size.y))
                            })
                            .map(|b| b.id);
                    }
                    n.reflow();
                    d.nodes.push(n);
                    changed = true;
                    *tool = Tool::Select;
                }
                Tool::Connect(kind) => {
                    self.connection_drag = None;
                    if let Some(node) = connection_target(d, &sc, w, 7. / d.view.zoom) {
                        if self.connection.is_none() {
                            self.connection_drag = Some(node);
                        }
                        changed |= self.connect_to(d, node, kind);
                    }
                }
                Tool::Select => {
                    let route = route_handles
                        .iter()
                        .find(|(_, _, point, _)| {
                            (point.x - w.x).hypot(point.y - w.y) < 10. / d.view.zoom
                        })
                        .map(|(id, i, _, points)| (*id, *i, points.clone()));
                    let resize = if self.selection.len() == 1 {
                        self.selection.iter().next().and_then(|id| {
                            sc.boxes
                                .get(id)
                                .filter(|r| {
                                    d.nodes.iter().any(|n| n.id == *id)
                                        && ((r.pos.x + r.size.x - w.x).abs() < 12. / d.view.zoom
                                            && (r.pos.y + r.size.y - w.y).abs() < 12. / d.view.zoom)
                                })
                                .map(|_| *id)
                        })
                    } else {
                        None
                    };
                    if resize.is_none() && route.is_none() {
                        if let Some(id) = hit {
                            if additive {
                                if !self.selection.insert(id) {
                                    self.selection.remove(&id);
                                }
                            } else if !self.selection.contains(&id) {
                                self.selection.clear();
                                self.selection.insert(id);
                            }
                        } else if !additive {
                            self.selection.clear();
                        }
                    }
                    let original = d.nodes.iter().map(|n| (n.id, n.layout)).collect();
                    self.drag = Some(Drag {
                        start: w,
                        last: w,
                        original,
                        marquee: hit.is_none() && resize.is_none() && route.is_none(),
                        route,
                        resize,
                        additive,
                        initial_selection: self.selection.clone(),
                    });
                }
            }
        }
        if (response.dragged_by(PointerButton::Primary)
            || response.drag_stopped_by(PointerButton::Primary))
            && let (Some(pos), Some(drag)) = (pointer, self.drag.as_mut())
        {
            let w = screen_to_world(pos, origin, d.view);
            if drag.marquee {
                let r = Layout::new(
                    w.x.min(drag.start.x),
                    w.y.min(drag.start.y),
                    (w.x - drag.start.x).abs(),
                    (w.y - drag.start.y).abs(),
                );
                self.selection = if drag.additive {
                    drag.initial_selection.clone()
                } else {
                    HashSet::new()
                };
                for (id, b) in &sc.selection_boxes() {
                    if r.contains(b.center()) {
                        self.selection.insert(*id);
                    }
                }
                let min = world_to_screen(r.pos, origin, d.view);
                painter.rect_stroke(
                    Rect::from_min_size(
                        min,
                        egui::vec2(r.size.x * d.view.zoom, r.size.y * d.view.zoom),
                    ),
                    0.,
                    Stroke::new(1_f32, Color32::from_rgb(63, 111, 235)),
                    egui::StrokeKind::Inside,
                );
            } else if let Some((id, index, points)) = &drag.route {
                if let Some(r) = d.relations.iter_mut().find(|r| r.id == *id) {
                    r.waypoints = points.clone();
                    r.waypoints[*index] = w;
                    r.bend = None;
                    changed = true;
                }
            } else if let Some(id) = drag.resize {
                if let (Some(n), Some(old)) = (
                    d.nodes.iter_mut().find(|n| n.id == id),
                    drag.original.get(&id),
                ) {
                    n.sizing = SizeMode::Manual;
                    n.layout.size = Point::new(
                        (old.size.x + w.x - drag.start.x).max(50.),
                        (old.size.y + w.y - drag.start.y).max(50.),
                    );
                    changed = true;
                }
            } else if d.kind != DiagramKind::Sequence {
                let target = if self.snap {
                    Point::new(
                        ((w.x - drag.start.x) / 20.).round() * 20.,
                        ((w.y - drag.start.y) / 20.).round() * 20.,
                    )
                } else {
                    Point::new(w.x - drag.start.x, w.y - drag.start.y)
                };
                let previous = Point::new(drag.last.x - drag.start.x, drag.last.y - drag.start.y);
                let delta = Point::new(target.x - previous.x, target.y - previous.y);
                if delta.x != 0. || delta.y != 0. {
                    for n in &mut d.nodes {
                        if let Some(original) = drag.original.get(&n.id) {
                            n.layout = *original;
                        }
                    }
                    commands::move_nodes(d, &self.selection, target);
                    changed = true;
                }
                drag.last = Point::new(drag.start.x + target.x, drag.start.y + target.y);
            }
        }
        if ui.input(|i| i.pointer.button_released(PointerButton::Primary))
            && let Some(source) = self.connection_drag.take()
            && self.connection == Some(source)
            && response.drag_stopped_by(PointerButton::Primary)
            && response.contains_pointer()
            && let Tool::Connect(kind) = *tool
            && let Some(pos) = pointer
            && let Some(target) = connection_target(
                d,
                &sc,
                screen_to_world(pos, origin, d.view),
                7. / d.view.zoom,
            )
            && target != source
        {
            changed |= self.connect_to(d, target, kind);
        }
        if ui.input(|i| i.pointer.button_released(PointerButton::Primary))
            && self.drag.take().is_some()
        {
            self.finished_drag = true;
        }
        let sc = scene::build(d);
        scene::paint(&sc, &painter, origin, d.view.pan, d.view.zoom);
        let mut hover_reason = None;
        if let Tool::Connect(kind) = *tool {
            let hovered = pointer
                .filter(|_| response.contains_pointer())
                .and_then(|pos| {
                    connection_target(
                        d,
                        &sc,
                        screen_to_world(pos, origin, d.view),
                        7. / d.view.zoom,
                    )
                });
            for node in &d.nodes {
                let result = if let Some(source) = self.connection {
                    crate::validation::relation(d, &Relation::new(source, node.id, kind))
                } else {
                    crate::validation::connection_start(d, node.id, kind)
                };
                let is_hovered = hovered == Some(node.id);
                let color = if result.is_ok() {
                    Color32::from_rgb(35, 146, 105)
                } else if is_hovered {
                    Color32::from_rgb(190, 53, 53)
                } else {
                    Color32::from_gray(180)
                };
                let min = world_to_screen(node.layout.pos, origin, d.view);
                painter.rect_stroke(
                    Rect::from_min_size(
                        min,
                        egui::vec2(node.layout.size.x, node.layout.size.y) * d.view.zoom,
                    )
                    .expand(4.),
                    4.,
                    Stroke::new(if is_hovered { 2.5_f32 } else { 1.5_f32 }, color),
                    egui::StrokeKind::Outside,
                );
                if is_hovered && ui.is_enabled() {
                    ui.ctx().set_cursor_icon(if result.is_ok() {
                        egui::CursorIcon::Crosshair
                    } else {
                        egui::CursorIcon::NotAllowed
                    });
                    hover_reason = result.err();
                }
            }
        }
        for (_, _, point, _) in &route_handles {
            painter.circle_filled(
                world_to_screen(*point, origin, d.view),
                5.,
                Color32::from_rgb(63, 111, 235),
            );
        }
        if self.guides && response.dragged_by(PointerButton::Primary) {
            for n in d.nodes.iter().filter(|n| self.selection.contains(&n.id)) {
                for other in d.nodes.iter().filter(|n| !self.selection.contains(&n.id)) {
                    for (a, b, vertical) in [
                        (n.layout.pos.x, other.layout.pos.x, true),
                        (n.layout.center().x, other.layout.center().x, true),
                        (n.layout.pos.y, other.layout.pos.y, false),
                        (n.layout.center().y, other.layout.center().y, false),
                    ] {
                        if (a - b).abs() < 5. / d.view.zoom {
                            let pts = if vertical {
                                [Point::new(b, -100000.), Point::new(b, 100000.)]
                            } else {
                                [Point::new(-100000., b), Point::new(100000., b)]
                            };
                            painter.line_segment(
                                [
                                    world_to_screen(pts[0], origin, d.view),
                                    world_to_screen(pts[1], origin, d.view),
                                ],
                                Stroke::new(1_f32, Color32::from_rgb(63, 111, 235)),
                            );
                        }
                    }
                }
            }
        }
        if let Tool::Connect(kind) = *tool
            && let (Some(source), Some(pos)) = (self.connection, pointer)
            && let Some(node) = d.nodes.iter().find(|n| n.id == source)
        {
            let w = screen_to_world(pos, origin, d.view);
            let target = connection_target(d, &sc, w, 7. / d.view.zoom);
            let valid = target
                .map(|target| crate::validation::relation(d, &Relation::new(source, target, kind)));
            let color = if valid.as_ref().is_some_and(Result::is_err) {
                Color32::DARK_RED
            } else {
                Color32::from_rgb(63, 111, 235)
            };
            let start = crate::routing::anchor(node, w);
            painter.line_segment(
                [world_to_screen(start, origin, d.view), pos],
                Stroke::new(2_f32, color),
            );
            if let Some(target) = target
                && let Some(r) = sc.boxes.get(&target)
            {
                let a = world_to_screen(r.pos, origin, d.view);
                painter.rect_stroke(
                    Rect::from_min_size(
                        a,
                        egui::vec2(r.size.x * d.view.zoom, r.size.y * d.view.zoom),
                    )
                    .expand(4.),
                    0.,
                    Stroke::new(2_f32, color),
                    egui::StrokeKind::Outside,
                );
            }
            if let Some(Err(reason)) = valid {
                response.clone().on_hover_text(reason);
            }
        }
        if sc.items.is_empty() {
            painter.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                if d.kind == DiagramKind::Sequence {
                    "‘시퀀스 추가’에서 참여자를 만든 뒤 메시지를 추가하세요"
                } else {
                    "‘요소 추가’에서 도구를 고른 뒤 캔버스를 클릭하세요"
                },
                egui::FontId::proportional(15.),
                Color32::from_gray(105),
            );
        }
        response.context_menu(|ui| {
            for (label, action) in [
                ("복사", ContextAction::Copy),
                ("붙여넣기", ContextAction::Paste),
                ("복제", ContextAction::Duplicate),
                ("삭제", ContextAction::Delete),
            ] {
                if ui.button(label).clicked() {
                    self.context_action = Some(action);
                    ui.close();
                }
            }
            if ui.button("전체 화면 맞춤").clicked() {
                fit(d, rect.size());
                ui.close();
            }
        });
        for id in &self.selection {
            if let Some(r) = sc.boxes.get(id) {
                let min = world_to_screen(r.pos, origin, d.view);
                let max = world_to_screen(
                    Point::new(r.pos.x + r.size.x, r.pos.y + r.size.y),
                    origin,
                    d.view,
                );
                painter.rect_stroke(
                    Rect::from_min_max(min, max).expand(4.),
                    0.,
                    Stroke::new(1.5_f32, Color32::from_rgb(63, 111, 235)),
                    egui::StrokeKind::Outside,
                );
                if d.nodes.iter().any(|n| n.id == *id) {
                    painter.rect_filled(
                        Rect::from_center_size(max, egui::vec2(9., 9.)),
                        1.,
                        Color32::from_rgb(63, 111, 235),
                    );
                }
            } else {
                for item in sc.items.iter().filter(|item| item.owner == *id) {
                    if let scene::Primitive::Path { points, .. } = &item.primitive {
                        for w in points.windows(2) {
                            painter.line_segment(
                                [
                                    world_to_screen(w[0], origin, d.view),
                                    world_to_screen(w[1], origin, d.view),
                                ],
                                Stroke::new(
                                    3_f32,
                                    Color32::from_rgba_unmultiplied(63, 111, 235, 120),
                                ),
                            );
                        }
                    }
                }
            }
        }
        if let Some(id) = self.connection
            && let Some(r) = sc.boxes.get(&id)
        {
            painter.circle_stroke(
                world_to_screen(r.center(), origin, d.view),
                15.,
                Stroke::new(2_f32, Color32::from_rgb(229, 155, 49)),
            );
        }
        // Paint inside the canvas, without an interactive overlay that could eat clicks.
        if matches!(*tool, Tool::Connect(_)) {
            let reason = hover_reason.as_ref().or(self.notice.as_ref());
            let text = reason
                .map(String::as_str)
                .unwrap_or(if self.connection.is_some() {
                    "초록 테두리: 연결 가능한 도착 요소 · 시작 → 도착 클릭 또는 드래그 · Esc 취소"
                } else {
                    "초록 테두리: 선택 가능한 시작 요소 · 시작 요소를 선택하세요 · Esc 취소"
                });
            let color = if reason.is_some() {
                Color32::from_rgb(160, 40, 40)
            } else {
                Color32::from_rgb(36, 95, 76)
            };
            let galley = painter.layout(
                text.into(),
                egui::FontId::proportional(13.),
                color,
                (rect.width() - 40.).max(1.),
            );
            let banner = Rect::from_min_size(
                rect.min + egui::vec2(8., 8.),
                galley.size() + egui::vec2(24., 16.),
            );
            painter.rect_filled(
                banner,
                6.,
                if reason.is_some() {
                    Color32::from_rgb(255, 235, 232)
                } else {
                    Color32::from_rgb(231, 245, 239)
                },
            );
            painter.galley(banner.min + egui::vec2(12., 8.), galley, color);
        }
        changed
    }
}
