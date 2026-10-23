use std::collections::HashSet;
use uml_studio::{
    commands::{self, Clipboard, History},
    model::*,
    routing,
    scene::{self, ExportOptions, Primitive},
    sequence_layout, storage, text_layout, validation,
};
fn classifier(name: &str, x: f32, y: f32) -> Node {
    let mut n = Node::new(NodeKind::Class, Point::new(x, y));
    n.name = name.into();
    n.sizing = SizeMode::Manual;
    n
}
fn participant(name: &str) -> Participant {
    Participant {
        id: id(),
        name: name.into(),
        actor: false,
        type_ref: None,
    }
}
fn message(source: Id, target: Id, kind: MessageKind, text: &str) -> Message {
    Message {
        id: id(),
        source,
        target,
        kind,
        text: text.into(),
        method_ref: None,
    }
}
fn sequence() -> Sequence {
    let a = participant("A");
    let b = participant("B");
    let c = participant("idle");
    let messages = vec![
        message(a.id, b.id, MessageKind::Sync, "call()"),
        message(b.id, b.id, MessageKind::Sync, "nested()"),
        message(b.id, a.id, MessageKind::Return, "result"),
        message(a.id, b.id, MessageKind::Sync, "again()"),
        message(b.id, a.id, MessageKind::Return, "done"),
    ];
    let activations = vec![
        Activation {
            id: id(),
            participant: b.id,
            start: messages[0].id,
            end: messages[2].id,
            automatic: false,
        },
        Activation {
            id: id(),
            participant: b.id,
            start: messages[1].id,
            end: messages[1].id,
            automatic: false,
        },
    ];
    Sequence {
        participants: vec![a, b, c],
        messages,
        activations,
        ..Default::default()
    }
}
#[test]
fn shared_validation_rejects_retargeting_and_cycles_without_mutation() {
    let mut d = Diagram::new(DiagramKind::Class, "types");
    d.nodes = vec![
        classifier("A", 0., 0.),
        classifier("B", 400., 0.),
        classifier("C", 800., 0.),
    ];
    let ids: Vec<_> = d.nodes.iter().map(|n| n.id).collect();
    d.add_relation(Relation::new(ids[0], ids[1], RelationKind::Generalization))
        .unwrap();
    d.add_relation(Relation::new(ids[1], ids[2], RelationKind::Generalization))
        .unwrap();
    let bad = Relation::new(ids[2], ids[0], RelationKind::Generalization);
    assert!(validation::relation(&d, &bad).unwrap_err().contains("순환"));
    assert!(d.add_relation(bad).is_err());
    let before = d.clone();
    let mut changed = d.relations[0].clone();
    changed.target = changed.source;
    assert!(d.update_relation(changed).is_err());
    assert_eq!(d, before);
    d.add_relation(Relation::new(ids[0], ids[0], RelationKind::Association))
        .unwrap();
}
#[test]
fn interface_generalization_and_class_realization_have_correct_direction() {
    let mut d = Diagram::new(DiagramKind::Class, "types");
    let a = Node::new(NodeKind::Interface, Point::default());
    let b = Node::new(NodeKind::Interface, Point::default());
    let c = classifier("implementation", 300., 0.);
    let (ai, bi, ci) = (a.id, b.id, c.id);
    d.nodes = vec![a, b, c];
    d.add_relation(Relation::new(ai, bi, RelationKind::Generalization))
        .unwrap();
    d.add_relation(Relation::new(ci, ai, RelationKind::Realization))
        .unwrap();
    assert!(
        d.add_relation(Relation::new(ai, ci, RelationKind::Realization))
            .is_err()
    );
}
#[test]
fn usecase_creation_and_editing_use_identical_rules() {
    let mut d = Diagram::new(DiagramKind::UseCase, "uc");
    let a = Node::new(NodeKind::Actor, Point::default());
    let b = Node::new(NodeKind::UseCase, Point::default());
    let c = Node::new(NodeKind::UseCase, Point::default());
    let (ai, bi, ci) = (a.id, b.id, c.id);
    d.nodes = vec![a, b, c];
    assert!(
        d.add_relation(Relation::new(ai, bi, RelationKind::Include))
            .is_err()
    );
    let r = Relation::new(bi, ci, RelationKind::Include);
    let rid = r.id;
    d.add_relation(r.clone()).unwrap();
    let mut edited = r;
    edited.source = ai;
    assert!(d.update_relation(edited).is_err());
    assert_eq!(d.relations[0].id, rid);
    assert_eq!(d.relations[0].source, bi);
}
#[test]
fn invalid_legacy_relationships_load_losslessly_for_repair() {
    let mut p = Project::new("old", DiagramKind::UseCase, "uc");
    let d = &mut p.diagrams[0];
    d.nodes = vec![
        Node::new(NodeKind::Actor, Point::default()),
        Node::new(NodeKind::UseCase, Point::new(300., 0.)),
    ];
    let r = Relation::new(d.nodes[0].id, d.nodes[1].id, RelationKind::Include);
    let rid = r.id;
    d.relations.push(r);
    p.format_version = 1;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.umlproj");
    std::fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    let mut loaded = storage::load(&path).unwrap();
    assert_eq!(loaded.format_version, 3);
    assert_eq!(loaded.diagrams[0].relations[0].id, rid);
    assert!(!loaded.diagnostics().is_empty());
    assert!(storage::save(&path, &loaded).is_err());
    let mut fixed = loaded.diagrams[0].relations[0].clone();
    fixed.kind = RelationKind::Association;
    loaded.diagrams[0].update_relation(fixed).unwrap();
    storage::save(&path, &loaded).unwrap();
    assert_eq!(storage::load(&path).unwrap(), loaded);
}
#[test]
fn v1_migration_preserves_ids_geometry_and_explicit_containment() {
    let original: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/v1.umlproj")).unwrap();
    let migrated = storage::load(std::path::Path::new("tests/fixtures/v1.umlproj")).unwrap();
    assert_eq!(migrated.id.to_string(), original["id"].as_str().unwrap());
    assert_eq!(migrated.format_version, 3);
    for (d, old) in migrated
        .diagrams
        .iter()
        .zip(original["diagrams"].as_array().unwrap())
    {
        assert_eq!(d.id.to_string(), old["id"].as_str().unwrap());
        for (n, oldn) in d.nodes.iter().zip(old["nodes"].as_array().unwrap()) {
            assert_eq!(n.id.to_string(), oldn["id"].as_str().unwrap());
            assert_eq!(
                n.layout,
                serde_json::from_value::<Layout>(oldn["layout"].clone()).unwrap()
            );
            assert_eq!(n.sizing, SizeMode::Manual);
        }
    }
    assert!(
        migrated.diagrams[0]
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::UseCase)
            .all(|n| n.boundary.is_some())
    );
}
#[test]
fn actor_scaling_changes_body_and_connection_points_with_fixed_text_size() {
    let mut d = Diagram::new(DiagramKind::UseCase, "actors");
    let mut a = Node::new(NodeKind::Actor, Point::new(0., 0.));
    a.name = "한글 이름을 사용하는 액터".into();
    a.layout.size = Point::new(160., 260.);
    a.reflow();
    let u = Node::new(NodeKind::UseCase, Point::new(600., 60.));
    let r = Relation::new(a.id, u.id, RelationKind::Association);
    d.nodes = vec![a.clone(), u];
    d.relations.push(r.clone());
    let arms = routing::actor_arms(a.layout, &a.name);
    let path = routing::path(&d, &r);
    assert_eq!(path[0], arms.1);
    let scene = scene::build(&d);
    let head = scene
        .items
        .iter()
        .find_map(|i| {
            if i.owner == a.id {
                if let Primitive::Ellipse(rect) = i.primitive {
                    Some(rect)
                } else {
                    None
                }
            } else {
                None
            }
        })
        .unwrap();
    assert!((head.size.x - 160. * 24. / 90.).abs() < 0.01);
    assert!(
        scene
            .items
            .iter()
            .filter_map(|i| if let Primitive::Text { size, .. } = i.primitive {
                Some(size)
            } else {
                None
            })
            .all(|s| s == 14. || s == 15.)
    );
}
#[test]
fn automatic_class_text_grows_and_manual_wrap_preserves_user_size() {
    let mut n = classifier("short", 0., 0.);
    n.sizing = SizeMode::Auto;
    n.reflow();
    let old = n.layout.size;
    n.name = "긴 한글 이름을 가진 정렬 요청 처리 서비스".repeat(3);
    n.methods.push(Member {
        name: "sort".into(),
        parameters: "values: List<정렬대상>, policy: 전략, options: 실행설정".into(),
        ..Default::default()
    });
    n.reflow();
    assert!(n.layout.size.x > old.x);
    assert!(n.layout.size.x >= text_width(&n.name, 16.) + 24.);
    n.sizing = SizeMode::Manual;
    n.layout.size = Point::new(220., 120.);
    let manual = n.layout;
    n.reflow();
    assert_eq!(n.layout, manual);
    let layout = text_layout::class_layout(&n, Some(220.));
    assert!(layout.overflow);
    assert!(
        layout
            .lines
            .iter()
            .all(|line| text_width(&line.text, line.size) <= 200.)
    );
    let before = layout.lines.len();
    n.methods.clear();
    n.attributes.clear();
    n.hide_empty = true;
    assert!(text_layout::class_layout(&n, Some(220.)).lines.len() < before);
}
#[test]
fn sequence_arrows_attach_to_edges_and_nested_calls_have_distinct_lanes() {
    let s = sequence();
    let l = sequence_layout::layout(&s);
    let base = l.activations[&s.activations[0].id];
    let nested = l.activations[&s.activations[1].id];
    assert!(nested.pos.x > base.pos.x);
    assert_eq!(l.messages[0].points.last().unwrap().x, base.pos.x);
    assert_eq!(l.messages[1].points[0].x, base.pos.x + base.size.x);
    assert_eq!(
        l.messages[1].points.last().unwrap().x,
        nested.pos.x + nested.size.x
    );
    assert_eq!(l.messages[2].points[0].x, base.pos.x);
    let mut reversed = s.clone();
    reversed.participants.swap(0, 1);
    let l = sequence_layout::layout(&reversed);
    let bar = l.activations[&s.activations[0].id];
    assert_eq!(
        l.messages[0].points.last().unwrap().x,
        bar.pos.x + bar.size.x
    );
    assert_eq!(l.messages[2].points[0].x, bar.pos.x + bar.size.x);
}
#[test]
fn fragments_scope_nest_and_reserve_safe_branch_gutters() {
    let mut s = sequence();
    let a = s.participants[0].id;
    let b = s.participants[1].id;
    let start = s.messages[0].id;
    let end = s.messages[4].id;
    s.fragments = vec![
        Fragment {
            id: id(),
            kind: FragmentKind::Alt,
            condition: "유효한 데이터가 있고 선택한 정렬 정책을 사용할 수 있는 경우".repeat(3),
            else_condition: "다른 조건".into(),
            start,
            end,
            split: Some(s.messages[3].id),
            participant_start: Some(a),
            participant_end: Some(b),
        },
        Fragment {
            id: id(),
            kind: FragmentKind::Loop,
            condition: "반복".into(),
            else_condition: String::new(),
            start: s.messages[1].id,
            end: s.messages[2].id,
            split: None,
            participant_start: Some(b),
            participant_end: Some(b),
        },
    ];
    let l = sequence_layout::layout(&s);
    let outer = &l.fragments[0];
    let inner = &l.fragments[1];
    assert!(outer.rect.contains(inner.rect.pos));
    assert!(outer.rect.contains(Point::new(
        inner.rect.pos.x + inner.rect.size.x,
        inner.rect.pos.y + inner.rect.size.y
    )));
    let split = outer.split.as_ref().unwrap().0;
    assert!(l.events.values().all(|y| (y - split).abs() > 15.));
    assert!(
        outer.rect.pos.x + outer.rect.size.x
            < l.participants[&s.participants[2].id].center().x - 20.
    );
    assert!(l.messages[0].labels[0].0.y > outer.rect.pos.y + outer.title.len() as f32 * 23.);
    let ids: Vec<_> = s.messages.iter().map(|m| m.id).collect();
    s.fragments[0].participant_start = Some(b);
    assert_eq!(ids, s.messages.iter().map(|m| m.id).collect::<Vec<_>>());
}
#[test]
fn long_multilingual_sequence_labels_use_available_width_and_separate_events() {
    let mut s = sequence();
    s.messages[0].text =
        "sort(values: List<정렬대상>, comparison: 사용자지정비교정책): 정렬된결과".repeat(5);
    s.messages[1].text = "자기 호출에 필요한 한글 설명과 매개변수".repeat(8);
    let l = sequence_layout::layout(&s);
    for m in &l.messages {
        assert!(!m.labels.is_empty());
        if m.id == s.messages[0].id {
            let span = (m.points[0].x - m.points.last().unwrap().x).abs();
            assert!(
                m.labels
                    .iter()
                    .all(|(_, s)| text_width(s, 14.) < span - 15.)
            );
        }
    }
    for pair in l.messages.windows(2) {
        assert!(
            pair[1].labels[0].0.y
                > pair[0]
                    .points
                    .iter()
                    .map(|p| p.y)
                    .fold(f32::NEG_INFINITY, f32::max)
        );
    }
}
#[test]
fn lifecycle_layout_and_diagnostics_survive_storage() {
    let mut p = Project::new("lifecycle", DiagramKind::Sequence, "flow");
    let a = participant("caller");
    let b = participant("created");
    let create = message(a.id, b.id, MessageKind::Create, "new");
    let call = message(a.id, b.id, MessageKind::Sync, "run");
    let destroy = message(a.id, b.id, MessageKind::Destroy, "delete");
    p.diagrams[0].sequence = Sequence {
        participants: vec![a.clone(), b.clone()],
        messages: vec![create.clone(), call, destroy.clone()],
        ..Default::default()
    };
    p.validate().unwrap();
    let l = sequence_layout::layout(&p.diagrams[0].sequence);
    assert_eq!(l.participants[&b.id].center().y, l.events[&create.id]);
    assert_eq!(l.lifelines[&b.id].1, l.events[&destroy.id]);
    assert_eq!(l.destroyed[&b.id].y, l.events[&destroy.id]);
    p.diagrams[0]
        .sequence
        .messages
        .push(message(a.id, b.id, MessageKind::Sync, "too late"));
    assert!(p.diagnostics().iter().any(|d| d.message.contains("소멸")));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recovery.umlproj");
    assert!(storage::save(&path, &p).is_err());
    storage::save_recovery(&path, &p).unwrap();
    let loaded = storage::load(&path).unwrap();
    assert_eq!(loaded.diagrams[0].sequence.messages.len(), 4);
}
#[test]
fn cross_diagram_ids_rename_roundtrip_and_deleted_targets_are_diagnosed() {
    let mut p = uml_studio::demo::review_project();
    p.validate().unwrap();
    let types = p.diagrams[2].sequence.participants[1].type_ref.unwrap();
    let node = p.diagrams[1]
        .nodes
        .iter_mut()
        .find(|n| n.id == types)
        .unwrap();
    node.name = "Renamed".into();
    node.methods[0].name = "renamed_sort".into();
    p.synchronize_links();
    assert_eq!(p.diagrams[2].sequence.participants[1].name, "Renamed");
    assert!(
        p.diagrams[2].sequence.messages[0]
            .text
            .contains("renamed_sort")
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.umlproj");
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
    let before = p.clone();
    p.diagrams[1].delete(&HashSet::from([types]));
    assert!(p.diagnostics().iter().any(|d| d.message.contains("타입")));
    let mut history = History::default();
    history.record(before.clone(), &p);
    history.undo(&mut p);
    assert_eq!(p, before);
}
#[test]
fn copied_sequence_preserves_scope_and_external_type_method_links() {
    let mut p = uml_studio::demo::review_project();
    let d = &mut p.diagrams[2];
    let selection = d.sequence.participants.iter().map(|p| p.id).collect();
    let old_messages = d.sequence.messages.clone();
    let clip = Clipboard::copy(d, &selection);
    clip.paste(d);
    assert_eq!(d.sequence.messages.len(), old_messages.len() * 2);
    assert_eq!(
        d.sequence.messages[old_messages.len()].method_ref,
        old_messages[0].method_ref
    );
    let copied = d.sequence.fragments.last().unwrap();
    assert!(
        d.sequence.participants[4..]
            .iter()
            .any(|p| Some(p.id) == copied.participant_start)
    );
    p.validate().unwrap();
}
#[test]
fn manual_routes_survive_endpoint_moves_and_copy_with_orthogonal_segments() {
    let mut d = Diagram::new(DiagramKind::Class, "routes");
    d.nodes = vec![classifier("A", 0., 0.), classifier("B", 600., 200.)];
    let mut r = Relation::new(d.nodes[0].id, d.nodes[1].id, RelationKind::Association);
    r.waypoints = vec![
        Point::new(300., 75.),
        Point::new(300., 400.),
        Point::new(500., 400.),
    ];
    d.add_relation(r.clone()).unwrap();
    commands::move_nodes(&mut d, &HashSet::from([r.target]), Point::new(200., 100.));
    assert_eq!(d.relations[0].waypoints, r.waypoints);
    let path = routing::path(&d, &r);
    assert!(
        path.windows(2)
            .all(|w| w[0].x == w[1].x || w[0].y == w[1].y)
    );
    let selection = d.nodes.iter().map(|n| n.id).collect();
    Clipboard::copy(&d, &selection).paste(&mut d);
    assert_eq!(d.relations[1].waypoints[0], Point::new(330., 105.));
}
#[test]
fn explicit_boundary_membership_is_stable_under_overlap_and_resize() {
    let mut d = Diagram::new(DiagramKind::UseCase, "membership");
    let mut a = Node::new(NodeKind::Boundary, Point::default());
    let mut b = a.clone();
    b.id = id();
    b.layout.pos.x = 50.;
    let mut child = Node::new(NodeKind::UseCase, Point::new(100., 100.));
    child.boundary = Some(b.id);
    let old = child.layout;
    let aid = a.id;
    a.layout.size = Point::new(1000., 1000.);
    d.nodes = vec![a, b, child];
    commands::move_nodes(&mut d, &HashSet::from([aid]), Point::new(100., 0.));
    assert_eq!(d.nodes[2].layout, old);
    let bid = d.nodes[1].id;
    commands::move_nodes(&mut d, &HashSet::from([bid]), Point::new(40., 30.));
    assert_eq!(
        d.nodes[2].layout.pos,
        Point::new(old.pos.x + 40., old.pos.y + 30.)
    );
}
#[test]
fn automatic_activation_ids_are_stable_and_manual_intervals_are_retained() {
    let mut s = sequence();
    s.auto_activate = true;
    let manual = s.activations.clone();
    s.regenerate_activations();
    let ids: Vec<_> = s.activations.iter().map(|a| a.id).collect();
    s.regenerate_activations();
    assert_eq!(ids, s.activations.iter().map(|a| a.id).collect::<Vec<_>>());
    assert!(manual.iter().all(|a| s.activations.contains(a)));
    s.auto_activate = false;
    s.regenerate_activations();
    assert_eq!(s.activations, manual);
}
#[test]
fn png_svg_and_preview_are_real_outputs_with_portable_outlined_text() {
    let dir = tempfile::tempdir().unwrap();
    let p = uml_studio::demo::review_project();
    for (i, d) in p.diagrams.iter().enumerate() {
        let drawing = scene::build(d);
        let opts = ExportOptions {
            scale: 1,
            margin: 32,
            transparent: true,
        };
        let (svg, w, h) = scene::svg(&drawing, &opts).unwrap();
        let preview = uml_studio::export::preview(&svg, w, h).unwrap();
        assert!(preview.width() <= 601 && preview.height() <= 341);
        let path = dir.path().join(format!("{i}.svg"));
        uml_studio::export::svg(&drawing, &path, &opts).unwrap();
        let bytes = std::fs::read_to_string(path).unwrap();
        assert!(bytes.contains("<path"));
        assert!(!bytes.contains("<text"));
        let path = dir.path().join(format!("{i}.png"));
        scene::export_png(&drawing, &path, &opts).unwrap();
        let png = resvg::tiny_skia::Pixmap::load_png(path).unwrap();
        assert_eq!((png.width(), png.height()), (w, h));
        assert_eq!(png.pixel(0, 0).unwrap().alpha(), 0);
    }
}

#[test]
fn quick_member_entry_parses_uml_and_rejects_incomplete_text() {
    let attr = Member::parse("- static capacity: usize = 1024", false).unwrap();
    assert_eq!(attr.visibility, "-");
    assert!(attr.is_static);
    assert_eq!(attr.value, "1024");
    let method = Member::parse(
        "# abstract sort(values: List<int>, policy: SortPolicy): Result<List<int>>",
        true,
    )
    .unwrap();
    assert!(method.is_abstract);
    assert_eq!(method.name, "sort");
    assert_eq!(method.parameters, "values: List<int>, policy: SortPolicy");
    assert_eq!(method.type_name, "Result<List<int>>");
    assert!(Member::parse("sort(values", true).is_err());
    assert!(Member::parse("missing type", false).is_err());
}

#[test]
fn dragging_sequence_rows_changes_order_without_replacing_event_ids() {
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    let ctx = egui::Context::default();
    let mut s = sequence();
    let old = s.participants[0].id;
    let target = s.participants[2].id;
    let mut selected = HashSet::new();
    let mut run = |events: Vec<Event>, s: &mut Sequence| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400., 600.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        let _ = uml_studio::properties::sequence_list(ui, s, &mut selected, "");
                    });
            },
        );
    };
    run(vec![], &mut s);
    let start = ctx
        .read_response(egui::Id::new(("sequence-order", old)))
        .unwrap()
        .rect
        .center();
    let end = ctx
        .read_response(egui::Id::new(("sequence-order", target)))
        .unwrap()
        .rect
        .center();
    let pressed = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(
        vec![Event::PointerMoved(start), pressed(start, true)],
        &mut s,
    );
    run(
        vec![Event::PointerMoved(start + egui::vec2(0., 10.))],
        &mut s,
    );
    run(vec![Event::PointerMoved(end)], &mut s);
    run(vec![], &mut s);
    run(vec![pressed(end, false)], &mut s);
    assert_eq!(s.participants[2].id, old);
    let mid = s.messages[0].id;
    let last = s.messages[4].id;
    let start = ctx
        .read_response(egui::Id::new(("sequence-order", mid)))
        .unwrap()
        .rect
        .center();
    let end = ctx
        .read_response(egui::Id::new(("sequence-order", last)))
        .unwrap()
        .rect
        .center();
    run(
        vec![Event::PointerMoved(start), pressed(start, true)],
        &mut s,
    );
    run(
        vec![Event::PointerMoved(start + egui::vec2(0., 10.))],
        &mut s,
    );
    run(vec![Event::PointerMoved(end)], &mut s);
    run(vec![], &mut s);
    run(vec![pressed(end, false)], &mut s);
    assert_eq!(s.messages[4].id, mid);
    let mut p = Project::new("drag", DiagramKind::Sequence, "flow");
    p.diagrams[0].sequence = s;
    p.validate().unwrap();
}
#[test]
fn canvas_route_handle_drag_is_an_actual_mouse_edit() {
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    let ctx = egui::Context::default();
    let mut d = Diagram::new(DiagramKind::Class, "routes");
    d.nodes = vec![classifier("A", 20., 20.), classifier("B", 600., 220.)];
    let r = Relation::new(d.nodes[0].id, d.nodes[1].id, RelationKind::Association);
    let rid = r.id;
    d.add_relation(r).unwrap();
    let mut state = uml_studio::canvas::CanvasState::new();
    state.selection.insert(rid);
    let mut tool = uml_studio::canvas::Tool::Select;
    let mut run = |events: Vec<Event>, d: &mut Diagram| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1100., 700.),
                )),
                events,
                ..Default::default()
            },
            |ctx| {
                egui::CentralPanel::default()
                    .frame(egui::Frame::NONE)
                    .show(ctx, |ui| {
                        state.ui(ui, d, &mut tool);
                    });
            },
        );
    };
    run(vec![], &mut d);
    let path = routing::path(&d, &d.relations[0]);
    let point = if path.len() == 2 {
        Point::new((path[0].x + path[1].x) / 2., (path[0].y + path[1].y) / 2.)
    } else {
        path[1]
    };
    let handle = uml_studio::canvas::world_to_screen(point, egui::Pos2::ZERO, d.view);
    let pressed = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(
        vec![Event::PointerMoved(handle), pressed(handle, true)],
        &mut d,
    );
    let end = handle + egui::vec2(60., 40.);
    run(vec![Event::PointerMoved(end)], &mut d);
    run(vec![pressed(end, false)], &mut d);
    assert!(!d.relations[0].waypoints.is_empty());
    assert_eq!(
        d.relations[0].waypoints[0],
        Point::new(point.x + 60., point.y + 40.)
    );
}

#[test]
fn copying_one_message_does_not_duplicate_other_messages_between_its_participants() {
    let mut d = Diagram::new(DiagramKind::Sequence, "copy");
    d.sequence = sequence();
    let clip = Clipboard::copy(&d, &HashSet::from([d.sequence.messages[0].id]));
    let before = d.sequence.messages.len();
    clip.paste(&mut d);
    assert_eq!(d.sequence.messages.len(), before + 1);
    assert_eq!(d.sequence.participants.len(), 5);
}
#[test]
fn copying_a_boundary_copies_explicit_children_and_their_internal_relations() {
    let mut p = uml_studio::app::example_project();
    let d = &mut p.diagrams[0];
    let parent = d.nodes[0].id;
    let count = d.nodes.len();
    let selected = Clipboard::copy(d, &HashSet::from([parent])).paste(d);
    assert_eq!(selected.len(), 3);
    assert_eq!(d.nodes.len(), count + 3);
    let new_parent = d.nodes[count].id;
    assert_ne!(new_parent, parent);
    assert!(
        d.nodes[count + 1..]
            .iter()
            .all(|n| n.boundary == Some(new_parent))
    );
    p.validate().unwrap();
}

#[test]
fn migrated_class_compartments_still_fit_original_saved_sizes() {
    let p = storage::load(std::path::Path::new("tests/fixtures/v1.umlproj")).unwrap();
    for n in p
        .diagrams
        .iter()
        .flat_map(|d| &d.nodes)
        .filter(|n| n.is_classifier())
    {
        let layout = text_layout::class_layout(n, Some(n.layout.size.x));
        assert!(
            !layout.overflow,
            "{} should still fit its existing box",
            n.name
        );
    }
}

#[test]
fn actor_generalization_uses_a_real_limb_and_avoids_the_name_below_it() {
    let mut d = Diagram::new(DiagramKind::UseCase, "actors");
    let mut parent = Node::new(NodeKind::Actor, Point::new(40., 40.));
    parent.name = "상위 액터의 긴 한글 이름".into();
    parent.reflow();
    let child = Node::new(NodeKind::Actor, Point::new(40., 440.));
    let r = Relation::new(child.id, parent.id, RelationKind::Generalization);
    d.nodes = vec![parent.clone(), child];
    d.add_relation(r.clone()).unwrap();
    let path = routing::path(&d, &r);
    let end = *path.last().unwrap();
    let arms = routing::actor_arms(parent.layout, &parent.name);
    assert!(end == arms.0 || end == arms.1);
    assert_eq!(path[path.len() - 2].y, end.y);
    assert!(
        path[path.len() - 2].x > parent.layout.pos.x + parent.layout.size.x
            || path[path.len() - 2].x < parent.layout.pos.x
    );
}

#[test]
fn recreating_a_destroyed_identity_is_diagnosed_instead_of_hiding_a_second_lifetime() {
    let mut p = Project::new("lifecycle", DiagramKind::Sequence, "flow");
    let a = participant("caller");
    let b = participant("object");
    p.diagrams[0].sequence = Sequence {
        participants: vec![a.clone(), b.clone()],
        messages: vec![
            message(a.id, b.id, MessageKind::Create, "new"),
            message(a.id, b.id, MessageKind::Destroy, "delete"),
            message(a.id, b.id, MessageKind::Create, "new again"),
        ],
        ..Default::default()
    };
    assert!(
        p.diagnostics()
            .iter()
            .any(|d| d.message.contains("두 번 생성"))
    );
}
