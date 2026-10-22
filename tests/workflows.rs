use std::collections::HashSet;
use uml_studio::{
    app::example_project,
    canvas,
    commands::{Clipboard, History, move_nodes},
    model::*,
    scene::{self, ExportOptions},
    storage,
};
#[test]
fn project_roundtrip_preserves_ids_and_relationships() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("정렬.umlproj");
    let p = example_project();
    p.validate().unwrap();
    storage::save(&path, &p).unwrap();
    let loaded = storage::load(&path).unwrap();
    assert_eq!(p, loaded);
    assert_eq!(loaded.diagrams.len(), 3);
    let mut edited = loaded;
    edited.name = "수정한 프로젝트".into();
    storage::save(&path, &edited).unwrap();
    assert_eq!(storage::load(&path).unwrap(), edited);
    assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
}
#[test]
fn move_delete_undo_redo_restores_connected_elements() {
    let mut p = example_project();
    let mut h = History::default();
    let original = p.clone();
    let selected = HashSet::from([p.diagrams[1].nodes[0].id]);
    move_nodes(&mut p.diagrams[1], &selected, Point::new(100., 70.));
    h.record(original.clone(), &p);
    let moved = p.clone();
    h.undo(&mut p);
    assert_eq!(p, original);
    h.redo(&mut p);
    assert_eq!(p, moved);
    p.diagrams[1].delete(&selected);
    assert!(
        p.diagrams[1]
            .relations
            .iter()
            .all(|r| !selected.contains(&r.source) && !selected.contains(&r.target))
    );
    h.record(moved.clone(), &p);
    let deleted = p.clone();
    h.undo(&mut p);
    assert_eq!(p, moved);
    h.redo(&mut p);
    assert_eq!(p, deleted);
    p.validate().unwrap();
}
#[test]
fn copy_paste_reissues_ids_and_keeps_internal_relationships() {
    let mut p = example_project();
    let d = &mut p.diagrams[1];
    let selected = d.nodes.iter().map(|n| n.id).collect();
    let original_nodes = d.nodes.clone();
    let original_relations = d.relations.clone();
    let clipboard = Clipboard::copy(d, &selected);
    let new_ids = clipboard.paste(d);
    assert_eq!(new_ids.len(), 3);
    assert_eq!(d.nodes.len(), 6);
    assert_eq!(d.relations.len(), 4);
    for n in &d.nodes[3..] {
        assert!(!original_nodes.iter().any(|old| old.id == n.id));
        for m in n.attributes.iter().chain(&n.methods) {
            assert!(
                !original_nodes
                    .iter()
                    .flat_map(|n| n.attributes.iter().chain(&n.methods))
                    .any(|old| old.id == m.id)
            );
        }
    }
    for r in &d.relations[2..] {
        assert!(new_ids.contains(&r.source) && new_ids.contains(&r.target));
        assert!(!original_relations.iter().any(|old| old.id == r.id));
    }
    p.validate().unwrap();
}
#[test]
fn sequence_copy_keeps_internal_event_references() {
    let mut p = example_project();
    let d = &mut p.diagrams[2];
    let selected = d.sequence.participants.iter().map(|p| p.id).collect();
    let clip = Clipboard::copy(d, &selected);
    clip.paste(d);
    assert_eq!(d.sequence.participants.len(), 6);
    assert_eq!(d.sequence.messages.len(), 10);
    assert_eq!(d.sequence.activations.len(), 4);
    assert_eq!(d.sequence.fragments.len(), 2);
    p.validate().unwrap();
}
#[test]
fn sequence_insert_delete_and_reorder_keep_references_consistent() {
    let mut p = example_project();
    let s = &mut p.diagrams[2].sequence;
    let start = s.messages[0].id;
    let mut inserted = s.messages[0].clone();
    inserted.id = id();
    inserted.text = "삽입".into();
    s.messages.insert(0, inserted);
    assert_eq!(s.activations[0].start, start);
    let selected = s.messages[3].id;
    s.remove_message(selected);
    assert!(s.messages.iter().all(|m| m.id != selected));
    assert!(
        s.fragments
            .iter()
            .all(|f| f.start != selected && f.end != selected)
    );
    s.messages.reverse();
    s.normalize();
    p.validate().unwrap();
    while let Some(mid) = p.diagrams[2].sequence.messages.first().map(|m| m.id) {
        p.diagrams[2].sequence.remove_message(mid);
        p.validate().unwrap();
    }
    assert!(p.diagrams[2].sequence.activations.is_empty());
    assert!(p.diagrams[2].sequence.fragments.is_empty());
}
#[test]
fn removing_participant_cascades_and_can_be_undone() {
    let mut p = example_project();
    let before = p.clone();
    let participant = p.diagrams[2].sequence.participants[1].id;
    p.diagrams[2].delete(&HashSet::from([participant]));
    p.validate().unwrap();
    assert!(
        p.diagrams[2]
            .sequence
            .messages
            .iter()
            .all(|m| m.source != participant && m.target != participant)
    );
    let mut history = History::default();
    history.record(before.clone(), &p);
    history.undo(&mut p);
    assert_eq!(p, before);
}
#[test]
fn boundary_moves_contained_use_cases_once() {
    let mut p = example_project();
    let d = &mut p.diagrams[0];
    let boundary = d.nodes[0].id;
    let child = d.nodes[2].id;
    let before = d.nodes[2].layout;
    let actor = d.nodes[1].layout;
    move_nodes(d, &HashSet::from([boundary, child]), Point::new(50., 60.));
    assert_eq!(
        d.nodes[2].layout.pos,
        Point::new(before.pos.x + 50., before.pos.y + 60.)
    );
    assert_eq!(d.nodes[1].layout, actor);
}
#[test]
fn coordinate_transform_roundtrip_across_zoom_levels() {
    let origin = eframe::egui::pos2(210., 70.);
    let world = Point::new(-95.25, 1234.5);
    for zoom in [0.1, 0.5, 1., 2., 5.] {
        let view = View {
            pan: Point::new(-320., 180.),
            zoom,
        };
        let screen = canvas::world_to_screen(world, origin, view);
        let back = canvas::screen_to_world(screen, origin, view);
        assert!((world.x - back.x).abs() < 0.001);
        assert!((world.y - back.y).abs() < 0.001);
    }
}
#[test]
fn png_exports_all_diagrams_with_korean_and_ignores_view() {
    let dir = tempfile::tempdir().unwrap();
    let mut p = example_project();
    for (i, d) in p.diagrams.iter_mut().enumerate() {
        let scene_before = scene::build(d);
        let opts = ExportOptions {
            scale: 1,
            margin: 27,
            transparent: true,
        };
        let (svg, w, h) = scene::svg(&scene_before, &opts).unwrap();
        assert!(svg.contains("font-family=\"NanumGothic\""));
        d.view.zoom = 0.1;
        d.view.pan = Point::new(-9000., 50000.);
        assert_eq!(scene::svg(&scene::build(d), &opts).unwrap().0, svg);
        let path = dir.path().join(format!("{i}.png"));
        assert_eq!(
            scene::export_png(&scene::build(d), &path, &opts).unwrap(),
            (w, h)
        );
        let bytes = std::fs::read(path).unwrap();
        let pixmap = resvg::tiny_skia::Pixmap::decode_png(&bytes).unwrap();
        assert_eq!((pixmap.width(), pixmap.height()), (w, h));
        assert_eq!(pixmap.pixel(0, 0).unwrap().alpha(), 0);
        assert!(pixmap.pixels().iter().filter(|p| p.alpha() > 0).count() > 500);
        let (_, w2, h2) =
            scene::svg(&scene::build(d), &ExportOptions { scale: 2, ..opts }).unwrap();
        assert_eq!((w2, h2), (w * 2, h * 2));
    }
}
#[test]
fn invalid_export_and_files_fail_without_replacing_existing_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("project.umlproj");
    let mut p = example_project();
    storage::save(&path, &p).unwrap();
    let before = std::fs::read(&path).unwrap();
    p.format_version = 999;
    assert!(storage::save(&path, &p).is_err());
    assert_eq!(std::fs::read(&path).unwrap(), before);
    std::fs::write(&path, r#"{"format_version":999}"#).unwrap();
    assert!(
        storage::load(&path)
            .unwrap_err()
            .contains("지원하지 않는 프로젝트 버전")
    );
    std::fs::write(&path, "not JSON").unwrap();
    assert!(storage::load(&path).unwrap_err().contains("JSON"));
    let d = Diagram::new(DiagramKind::Class, "empty");
    assert!(scene::svg(&scene::build(&d), &ExportOptions::default()).is_err());
    let mut p = example_project();
    p.diagrams[0].nodes[0].layout.size = Point::new(30000., 30000.);
    assert!(scene::svg(&scene::build(&p.diagrams[0]), &ExportOptions::default()).is_err());
}
#[test]
fn duplicate_ids_and_dangling_references_are_rejected() {
    let mut p = example_project();
    p.diagrams[1].nodes[1].id = p.diagrams[1].nodes[0].id;
    assert!(p.validate().is_err());
    let mut p = example_project();
    p.diagrams[2].sequence.messages[0].target = id();
    assert!(p.validate().is_err());
}
#[test]
fn scene_hit_tests_pick_nodes_lines_and_do_not_swallow_boundary_interior() {
    let p = example_project();
    let d = &p.diagrams[0];
    let sc = scene::build(d);
    assert_eq!(sc.hit(d.nodes[2].layout.center(), 6.), Some(d.nodes[2].id));
    assert_eq!(sc.hit(Point::new(780., 380.), 6.), None);
    let d = &p.diagrams[1];
    let sc = scene::build(d);
    let relation = d.relations[0].id;
    assert_eq!(
        sc.hit(Point::new(500., d.nodes[1].layout.center().y), 6.),
        Some(relation)
    );
}

#[test]
fn viewport_navigation_is_not_an_undo_command() {
    let mut p = example_project();
    let mut history = History::default();
    let before = p.clone();
    p.diagrams[1].nodes[0].name = "Changed".into();
    history.record(before.clone(), &p);
    let after_edit = p.clone();
    p.diagrams[1].view.zoom = 0.5;
    p.diagrams[1].view.pan = Point::new(-300., 60.);
    history.record(after_edit, &p);
    history.undo(&mut p);
    assert_eq!(
        p.diagrams[1].nodes[0].name,
        before.diagrams[1].nodes[0].name
    );
    assert_eq!(p.diagrams[1].view.zoom, 0.5);
    history.redo(&mut p);
    assert_eq!(p.diagrams[1].nodes[0].name, "Changed");
    assert_eq!(p.diagrams[1].view.pan, Point::new(-300., 60.));
}
#[test]
fn korean_glyphs_are_present_in_export() {
    let dir = tempfile::tempdir().unwrap();
    let mut d = Diagram::new(DiagramKind::UseCase, "한글");
    let mut node = Node::new(NodeKind::UseCase, Point::new(0., 0.));
    node.name = "한글 정렬".into();
    d.nodes.push(node);
    let path = dir.path().join("korean.png");
    scene::export_png(
        &scene::build(&d),
        &path,
        &ExportOptions {
            scale: 1,
            margin: 10,
            transparent: false,
        },
    )
    .unwrap();
    let pixmap = resvg::tiny_skia::Pixmap::load_png(path).unwrap();
    let mut ink = 0;
    for y in 35..65 {
        for x in 50..150 {
            let p = pixmap.pixel(x, y).unwrap();
            if p.red() < 180 {
                ink += 1;
            }
        }
    }
    assert!(
        ink > 60,
        "Korean text area must contain glyph ink, got {ink}"
    );
}

#[test]
fn canvas_drag_selection_and_resize_work_after_zoom() {
    use eframe::egui::{self, Event, Modifiers, PointerButton};
    let ctx = egui::Context::default();
    let mut d = Diagram::new(DiagramKind::Class, "test");
    d.view = View {
        pan: Point::new(40., 40.),
        zoom: 1.5,
    };
    let n = Node::new(NodeKind::Class, Point::new(100., 100.));
    let nid = n.id;
    d.nodes.push(n);
    let mut state = canvas::CanvasState::new();
    let mut tool = canvas::Tool::Select;
    let mut run = |events: Vec<Event>, d: &mut Diagram, state: &mut canvas::CanvasState| {
        let _ = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(1000., 800.),
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
    run(vec![], &mut d, &mut state);
    let pos = egui::pos2(265., 235.);
    let pressed = |pos, pressed| Event::PointerButton {
        pos,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    };
    run(
        vec![Event::PointerMoved(pos), pressed(pos, true)],
        &mut d,
        &mut state,
    );
    let moved = pos + egui::vec2(90., 60.);
    run(vec![Event::PointerMoved(moved)], &mut d, &mut state);
    run(vec![pressed(moved, false)], &mut d, &mut state);
    assert!(state.selection.contains(&nid));
    assert_eq!(d.nodes[0].layout.pos, Point::new(160., 140.));
    assert!(state.finished_drag);
    let handle = canvas::world_to_screen(Point::new(390., 290.), egui::Pos2::ZERO, d.view);
    run(
        vec![Event::PointerMoved(handle), pressed(handle, true)],
        &mut d,
        &mut state,
    );
    let next = handle + egui::vec2(45., 30.);
    run(vec![Event::PointerMoved(next)], &mut d, &mut state);
    run(vec![pressed(next, false)], &mut d, &mut state);
    assert_eq!(d.nodes[0].layout.size, Point::new(260., 170.));
}

#[test]
fn copying_a_connection_alone_reissues_its_id_and_keeps_endpoints() {
    let mut p = example_project();
    let d = &mut p.diagrams[1];
    let original = d.relations[0].clone();
    let clip = Clipboard::copy(d, &HashSet::from([original.id]));
    let selected = clip.paste(d);
    let new = d.relations.last().unwrap();
    assert_ne!(new.id, original.id);
    assert_eq!((new.source, new.target), (original.source, original.target));
    assert!(selected.contains(&new.id));
    p.validate().unwrap();
}
