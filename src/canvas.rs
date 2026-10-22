use crate::{commands, model::*, scene};
use eframe::egui::{self, Color32, Key, PointerButton, Rect, Sense, Stroke};
use std::collections::{HashMap, HashSet};
#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Select,
    Add(NodeKind),
    Connect(RelationKind),
}
#[derive(Default)]
pub struct CanvasState {
    pub selection: HashSet<Id>,
    pub grid: bool,
    pub snap: bool,
    pub connection: Option<Id>,
    drag: Option<Drag>,
    pub finished_drag: bool,
}
struct Drag {
    start: Point,
    last: Point,
    original: HashMap<Id, Layout>,
    marquee: bool,
    resize: Option<Id>,
    additive: bool,
    initial_selection: HashSet<Id>,
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
impl CanvasState {
    pub fn new() -> Self {
        Self {
            grid: true,
            ..Default::default()
        }
    }
    pub fn clear(&mut self) {
        self.selection.clear();
        self.connection = None;
        self.drag = None;
    }
    pub fn ui(&mut self, ui: &mut egui::Ui, d: &mut Diagram, tool: &mut Tool) -> bool {
        let (response, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
        let rect = response.rect;
        let origin = rect.min;
        let mut changed = false;
        self.finished_drag = false;
        painter.rect_filled(rect, 0., Color32::from_rgb(247, 249, 252));
        let pointer = ui.input(|i| i.pointer.hover_pos());
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
        let primary_pressed = ui.input(|i| i.pointer.button_pressed(PointerButton::Primary));
        if response.hovered()
            && primary_pressed
            && let Some(pos) = pointer
        {
            let w = screen_to_world(pos, origin, d.view);
            let hit = sc.hit(w, 7. / d.view.zoom);
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
                    d.nodes.push(n);
                    changed = true;
                    *tool = Tool::Select;
                }
                Tool::Connect(kind) => {
                    if let Some(node) = hit.filter(|i| d.nodes.iter().any(|n| n.id == *i)) {
                        if let Some(source) = self.connection.take() {
                            let valid = if d.kind == DiagramKind::UseCase {
                                let a = d.nodes.iter().find(|n| n.id == source).unwrap().kind;
                                let b = d.nodes.iter().find(|n| n.id == node).unwrap().kind;
                                match kind {
                                    RelationKind::Include | RelationKind::Extend => {
                                        a == NodeKind::UseCase && b == NodeKind::UseCase
                                    }
                                    RelationKind::Generalization => {
                                        a == b && matches!(a, NodeKind::Actor | NodeKind::UseCase)
                                    }
                                    _ => matches!(
                                        (a, b),
                                        (NodeKind::Actor, NodeKind::UseCase)
                                            | (NodeKind::UseCase, NodeKind::Actor)
                                    ),
                                }
                            } else {
                                true
                            };
                            if valid {
                                let r = Relation::new(source, node, kind);
                                self.selection.clear();
                                self.selection.insert(r.id);
                                d.relations.push(r);
                                changed = true;
                            }
                        } else {
                            self.connection = Some(node);
                        }
                    }
                }
                Tool::Select => {
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
                    if resize.is_none() {
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
                        marquee: hit.is_none() && resize.is_none(),
                        resize,
                        additive,
                        initial_selection: self.selection.clone(),
                    });
                }
            }
        }
        if response.dragged_by(PointerButton::Primary)
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
            } else if let Some(id) = drag.resize {
                if let (Some(n), Some(old)) = (
                    d.nodes.iter_mut().find(|n| n.id == id),
                    drag.original.get(&id),
                ) {
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
            && self.drag.take().is_some()
        {
            self.finished_drag = true;
        }
        scene::paint(&scene::build(d), &painter, origin, d.view.pan, d.view.zoom);
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
        if response.hovered()
            && !ui.ctx().wants_keyboard_input()
            && ui.input(|i| i.key_pressed(Key::Escape))
        {
            *tool = Tool::Select;
            self.connection = None;
        }
        changed
    }
}
