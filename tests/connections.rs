use eframe::egui::{self, Event, Modifiers, PointerButton};
use uml_studio::{
    canvas::{CanvasState, Tool, world_to_screen},
    model::*,
    scene,
};

struct Canvas {
    ctx: egui::Context,
    state: CanvasState,
    tool: Tool,
    diagram: Diagram,
    time: f64,
}
impl Canvas {
    fn new(
        kind: DiagramKind,
        source: NodeKind,
        target: NodeKind,
        relation: RelationKind,
        zoom: f32,
    ) -> Self {
        let mut diagram = Diagram::new(kind, "connection test");
        diagram.nodes = vec![
            Node::new(source, Point::new(80., 120.)),
            Node::new(target, Point::new(520., 240.)),
        ];
        diagram.view.zoom = zoom;
        diagram.view.pan = Point::new(30., 40.);
        let mut canvas = Self {
            ctx: egui::Context::default(),
            state: CanvasState::new(),
            tool: Tool::Connect(relation),
            diagram,
            time: 0.,
        };
        canvas.run(vec![]);
        canvas
    }
    fn run(&mut self, events: Vec<Event>) -> bool {
        self.time += 0.05;
        let mut changed = false;
        let _ = self.ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1400., 900.),
                )),
                events,
                time: Some(self.time),
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        changed = self.state.ui(ui, &mut self.diagram, &mut self.tool);
                    });
            },
        );
        changed
    }
    fn point(&self, index: usize) -> egui::Pos2 {
        let node = &self.diagram.nodes[index];
        let point = if node.kind == NodeKind::Actor {
            // Inside the actor's selectable box, between its strokes.
            Point::new(node.layout.pos.x + 4., node.layout.pos.y + 35.)
        } else {
            node.layout.center()
        };
        world_to_screen(point, egui::Pos2::ZERO, self.diagram.view)
    }
    fn button(pos: egui::Pos2, pressed: bool) -> Event {
        Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        }
    }
    fn click(&mut self, pos: egui::Pos2) -> bool {
        self.run(vec![Event::PointerMoved(pos)]);
        let pressed = self.run(vec![Self::button(pos, true)]);
        let released = self.run(vec![Self::button(pos, false)]);
        pressed || released
    }
    fn drag(&mut self, from: egui::Pos2, to: egui::Pos2) -> bool {
        self.run(vec![Event::PointerMoved(from)]);
        self.run(vec![Self::button(from, true)]);
        self.run(vec![Event::PointerMoved(to)]);
        self.run(vec![Self::button(to, false)])
    }
}

fn cases() -> Vec<(DiagramKind, NodeKind, NodeKind, RelationKind)> {
    use DiagramKind::{Class as CD, UseCase as UC};
    use NodeKind::*;
    use RelationKind::*;
    vec![
        (UC, Actor, UseCase, Association),
        (UC, UseCase, Actor, Association),
        (UC, UseCase, UseCase, Include),
        (UC, UseCase, UseCase, Extend),
        (UC, Actor, Actor, Generalization),
        (UC, UseCase, UseCase, Generalization),
        (UC, Note, UseCase, Comment),
        (UC, Actor, Note, Comment),
        (CD, Class, Class, Association),
        (CD, Class, Class, Dependency),
        (CD, Class, AbstractClass, Generalization),
        (CD, Interface, Interface, Generalization),
        (CD, Class, Interface, Realization),
        (CD, AbstractClass, Interface, Realization),
        (CD, Class, Class, Aggregation),
        (CD, Class, Class, Composition),
        (CD, Enumeration, Class, Association),
        (CD, Note, Class, Comment),
        (CD, Interface, Note, Comment),
    ]
}

#[test]
fn every_supported_relation_can_be_created_by_clicking_at_different_zooms() {
    for (diagram, source, target, relation) in cases() {
        for zoom in [0.6, 1., 1.4] {
            let mut c = Canvas::new(diagram, source, target, relation, zoom);
            let ids = (c.diagram.nodes[0].id, c.diagram.nodes[1].id);
            assert!(!c.click(c.point(0)));
            assert_eq!(
                c.state.connection,
                Some(ids.0),
                "source not picked: {source:?}, zoom {zoom}"
            );
            assert!(c.click(c.point(1)), "{relation:?}: {source:?} → {target:?}");
            assert_eq!(c.diagram.relations.len(), 1);
            let r = &c.diagram.relations[0];
            assert_eq!((r.source, r.target, r.kind), (ids.0, ids.1, relation));
            assert!(c.state.selection.contains(&r.id));
            assert!(c.state.connection.is_none());
            assert!(c.state.edit_name.is_none());
        }
    }
}

#[test]
fn every_supported_relation_can_also_be_created_by_dragging() {
    for (diagram, source, target, relation) in cases() {
        let mut c = Canvas::new(diagram, source, target, relation, 0.8);
        let before: Vec<_> = c.diagram.nodes.iter().map(|n| n.layout).collect();
        assert!(
            c.drag(c.point(0), c.point(1)),
            "drag {relation:?}: {source:?} → {target:?}"
        );
        assert_eq!(c.diagram.relations.len(), 1);
        assert!(c.state.connection.is_none());
        assert_eq!(
            c.diagram.nodes.iter().map(|n| n.layout).collect::<Vec<_>>(),
            before
        );
    }
}

#[test]
fn existing_line_over_an_actor_does_not_intercept_the_connection_click() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::Actor,
        NodeKind::UseCase,
        RelationKind::Association,
        1.,
    );
    let source = &c.diagram.nodes[0];
    let (body, _) = uml_studio::routing::actor_body(source.layout, &source.name);
    let p = Point::new(
        body.pos.x + body.size.x - 2.,
        body.pos.y + body.size.y * 0.43,
    );
    let r = Relation::new(source.id, c.diagram.nodes[1].id, RelationKind::Association);
    let rid = r.id;
    c.diagram.add_relation(r).unwrap();
    assert_eq!(scene::build(&c.diagram).hit(p, 7.), Some(rid));
    let pos = world_to_screen(p, egui::Pos2::ZERO, c.diagram.view);
    c.click(pos);
    assert_eq!(c.state.connection, Some(c.diagram.nodes[0].id));
    assert!(c.click(c.point(1)));
    assert_eq!(c.diagram.relations.len(), 2);
}

#[test]
fn invalid_destination_keeps_the_source_for_a_valid_retry() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::UseCase,
        NodeKind::Actor,
        RelationKind::Include,
        1.,
    );
    let mut valid = Node::new(NodeKind::UseCase, Point::new(800., 120.));
    valid.reflow();
    c.diagram.nodes.push(valid);
    c.click(c.point(0));
    assert!(!c.click(c.point(1)));
    assert!(c.diagram.relations.is_empty());
    assert!(c.state.notice.as_ref().unwrap().contains("유스케이스"));
    assert_eq!(c.state.connection, Some(c.diagram.nodes[0].id));
    assert!(c.click(c.point(2)));
    assert!(c.state.notice.is_none());
}

#[test]
fn self_association_works_without_triggering_name_editing() {
    let mut c = Canvas::new(
        DiagramKind::Class,
        NodeKind::Class,
        NodeKind::Class,
        RelationKind::Association,
        1.,
    );
    c.click(c.point(0));
    assert!(c.click(c.point(0)));
    assert_eq!(c.diagram.relations[0].source, c.diagram.relations[0].target);
    assert!(c.state.edit_name.is_none());
}

#[test]
fn leaving_the_connection_tool_clears_its_pending_source() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::UseCase,
        NodeKind::UseCase,
        RelationKind::Include,
        1.,
    );
    c.click(c.point(0));
    c.tool = Tool::Select;
    c.run(vec![]);
    assert!(c.state.connection.is_none());
    c.tool = Tool::Connect(RelationKind::Extend);
    assert!(!c.click(c.point(1)));
    assert!(c.diagram.relations.is_empty());
    assert_eq!(c.state.connection, Some(c.diagram.nodes[1].id));
}

#[test]
fn escape_cancels_connection_even_with_pointer_outside_canvas() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::UseCase,
        NodeKind::UseCase,
        RelationKind::Include,
        1.,
    );
    c.click(c.point(0));
    c.run(vec![
        Event::PointerMoved(egui::pos2(-10., -10.)),
        Event::Key {
            key: egui::Key::Escape,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        },
    ]);
    assert!(c.state.connection.is_none());
    assert!(matches!(c.tool, Tool::Select));
}

#[test]
fn invalid_relation_pairs_report_errors_without_changing_the_diagram() {
    use DiagramKind::{Class as CD, UseCase as UC};
    use NodeKind::*;
    use RelationKind::*;
    for (diagram, source, target, relation) in [
        (UC, Actor, Actor, Association),
        (UC, UseCase, UseCase, Association),
        (UC, Actor, UseCase, Include),
        (UC, UseCase, Actor, Extend),
        (UC, Actor, UseCase, Generalization),
        (UC, UseCase, UseCase, Dependency),
        (UC, UseCase, UseCase, Realization),
        (UC, Actor, UseCase, Comment),
        (CD, Interface, Class, Realization),
        (CD, Class, Class, Realization),
        (CD, Class, Interface, Generalization),
        (CD, Enumeration, Class, Generalization),
        (CD, Note, Class, Association),
        (CD, Class, Class, Include),
        (CD, Class, Class, Extend),
        (CD, Class, Class, Comment),
    ] {
        let mut c = Canvas::new(diagram, source, target, relation, 1.);
        let before = c.diagram.clone();
        c.click(c.point(0));
        assert!(
            !c.click(c.point(1)),
            "invalid {relation:?}: {source:?} → {target:?}"
        );
        assert_eq!(c.diagram, before);
        assert!(c.state.notice.as_ref().is_some_and(|s| !s.is_empty()));
        assert_eq!(c.state.connection, Some(before.nodes[0].id));
    }
}

#[test]
fn invalid_drag_can_be_retried_and_releasing_outside_creates_no_relation() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::UseCase,
        NodeKind::Actor,
        RelationKind::Include,
        1.,
    );
    assert!(!c.drag(c.point(0), c.point(1)));
    assert!(
        c.state
            .notice
            .as_ref()
            .is_some_and(|s| s.contains("유스케이스"))
    );
    assert!(c.diagram.relations.is_empty());
    c.diagram.nodes[1].kind = NodeKind::UseCase;
    assert!(c.click(c.point(1)));
    assert_eq!(c.diagram.relations.len(), 1);

    let mut c = Canvas::new(
        DiagramKind::Class,
        NodeKind::Class,
        NodeKind::Class,
        RelationKind::Association,
        1.,
    );
    assert!(!c.drag(c.point(0), egui::pos2(-20., -20.)));
    assert!(c.diagram.relations.is_empty());
    assert!(c.click(c.point(1)));
    assert_eq!(c.diagram.relations.len(), 1);
}

#[test]
fn system_boundary_interior_does_not_capture_connection_endpoints() {
    let mut c = Canvas::new(
        DiagramKind::UseCase,
        NodeKind::Actor,
        NodeKind::UseCase,
        RelationKind::Association,
        1.,
    );
    let mut boundary = Node::new(NodeKind::Boundary, Point::new(50., 60.));
    boundary.layout.size = Point::new(900., 600.);
    c.diagram.nodes.push(boundary);
    c.click(c.point(0));
    assert!(c.click(c.point(1)));
    let r = &c.diagram.relations[0];
    assert_eq!(
        (r.source, r.target),
        (c.diagram.nodes[0].id, c.diagram.nodes[1].id)
    );
}

#[test]
fn each_created_relation_can_be_retargeted_undone_saved_and_loaded() {
    use uml_studio::{commands::History, storage};
    let dir = tempfile::tempdir().unwrap();
    for (diagram, source, target, kind) in cases() {
        let mut c = Canvas::new(diagram, source, target, kind, 1.);
        let mut project = Project::new("connections", diagram, "test");
        project.diagrams[0] = c.diagram.clone();
        let before = project.clone();
        c.click(c.point(0));
        assert!(c.click(c.point(1)));
        project.diagrams[0] = c.diagram;
        let mut history = History::default();
        history.record(before.clone(), &project);
        let created = project.clone();
        history.undo(&mut project);
        assert_eq!(project, before);
        history.redo(&mut project);
        assert_eq!(project, created);

        let d = &mut project.diagrams[0];
        let mut new_target = d.nodes[1].clone();
        new_target.id = id();
        new_target.layout.pos = Point::new(800., 400.);
        let mut edited = d.relations[0].clone();
        edited.target = new_target.id;
        edited.name = "수정한 연결".into();
        d.nodes.push(new_target);
        d.update_relation(edited.clone()).unwrap();
        assert_eq!(d.relations[0], edited);
        edited.target = id();
        let before_invalid = d.clone();
        assert!(d.update_relation(edited).is_err());
        assert_eq!(d, &before_invalid);
        let path = dir.path().join("relations.umlproj");
        storage::save(&path, &project).unwrap();
        assert_eq!(storage::load(&path).unwrap(), project);
    }
}
