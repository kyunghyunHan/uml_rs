use std::{collections::HashSet, fs, sync::Arc};
use uml_studio::{
    commands::{self, Clipboard, History},
    model::*,
    routing, scene, storage,
};

#[test]
fn concurrent_editors_refuse_to_overwrite_and_missing_original_is_preserved() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("한글 프로젝트.umlproj");
    let p = Project::new("원본", DiagramKind::Class, "설계");
    storage::save(&path, &p).unwrap();
    let (mut first, rev1) = storage::load_tracked(&path).unwrap();
    let (mut second, rev2) = storage::load_tracked(&path).unwrap();
    first.name = "먼저 저장".into();
    let revision = storage::save_tracked(&path, &first, Some(&rev1)).unwrap();
    second.name = "나중 저장".into();
    assert!(
        storage::save_tracked(&path, &second, Some(&rev2))
            .err()
            .unwrap()
            .contains("변경")
    );
    assert_eq!(storage::load(&path).unwrap().name, "먼저 저장");
    fs::remove_file(&path).unwrap();
    assert!(storage::save_tracked(&path, &first, Some(&revision)).is_err());
    assert!(!path.exists());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
}
#[test]
fn save_lock_conflict_does_not_modify_original_or_remove_other_lock() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("p.umlproj");
    let p = Project::new("p", DiagramKind::Class, "d");
    storage::save(&path, &p).unwrap();
    let original = fs::read(&path).unwrap();
    let lock = dir.path().join("p.umlproj.uml-lock");
    fs::write(&lock, "other writer").unwrap();
    assert!(storage::save(&path, &p).is_err());
    assert_eq!(fs::read(&path).unwrap(), original);
    assert_eq!(fs::read(&lock).unwrap(), b"other writer");
}
#[test]
fn recovery_sessions_and_legacy_snapshots_remain_independent() {
    let dir = tempfile::tempdir().unwrap();
    let a = storage::RecoveryStore::new(dir.path().into());
    let b = storage::RecoveryStore::new(dir.path().into());
    let p = Project::new("보존", DiagramKind::UseCase, "흐름");
    a.save(&p).unwrap();
    b.save(&p).unwrap();
    let legacy = dir.path().join("recovery.umlproj");
    storage::save_recovery(&legacy, &p).unwrap();
    assert_eq!(a.candidates().len(), 2);
    a.clear().unwrap();
    a.clear().unwrap();
    assert_eq!(a.candidates().len(), 2);
    for candidate in a.candidates() {
        assert_eq!(storage::load(&candidate).unwrap(), p);
    }
    b.clear().unwrap();
    assert!(legacy.exists());
}
#[test]
fn malformed_versions_duplicate_ids_and_oversize_files_are_rejected() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("bad.umlproj");
    for bytes in [
        b"{".as_slice(),
        b"null",
        b"[]",
        b"{\"format_version\":4}",
        b"{\"format_version\":-1}",
    ] {
        fs::write(&path, bytes).unwrap();
        assert!(storage::load(&path).is_err());
    }
    let mut p = Project::new("p", DiagramKind::Class, "d");
    let mut n = Node::new(NodeKind::Class, Point::default());
    n.id = p.id;
    p.diagrams[0].nodes.push(n);
    fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    assert!(storage::load(&path).unwrap_err().contains("중복"));
    let file = fs::File::create(&path).unwrap();
    file.set_len(32 * 1024 * 1024 + 1).unwrap();
    assert!(storage::load(&path).unwrap_err().contains("32 MiB"));
}
#[test]
fn failed_rename_cleans_temporary_file_and_missing_parent_fails() {
    let dir = tempfile::tempdir().unwrap();
    let target = dir.path().join("directory");
    fs::create_dir(&target).unwrap();
    assert!(storage::atomic_write(&target, b"data").is_err());
    assert!(target.is_dir());
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
    assert!(storage::atomic_write(&dir.path().join("missing/p"), b"data").is_err());
}
#[cfg(unix)]
#[test]
fn replacing_existing_file_keeps_its_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("private");
    fs::write(&path, b"old").unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o600)).unwrap();
    storage::atomic_write(&path, b"new").unwrap();
    assert_eq!(
        fs::metadata(path).unwrap().permissions().mode() & 0o777,
        0o600
    );
}
fn nested() -> Project {
    let mut p = Project::new("packages", DiagramKind::Class, "d");
    let root = Node::new(NodeKind::Package, Point::new(0., 0.));
    let mut child = Node::new(NodeKind::Package, Point::new(20., 20.));
    child.boundary = Some(root.id);
    let mut leaf = Node::new(NodeKind::Class, Point::new(40., 40.));
    leaf.boundary = Some(child.id);
    p.diagrams[0].nodes = vec![root, child, leaf];
    p
}
#[test]
fn nested_package_moves_once_and_cycles_are_diagnosed_without_destroying_recovery() {
    let mut p = nested();
    let before = p.clone();
    let selected = HashSet::from([p.diagrams[0].nodes[0].id, p.diagrams[0].nodes[1].id]);
    commands::move_nodes(&mut p.diagrams[0], &selected, Point::new(10., 15.));
    for (a, b) in before.diagrams[0].nodes.iter().zip(&p.diagrams[0].nodes) {
        assert_eq!(
            b.layout.pos,
            Point::new(a.layout.pos.x + 10., a.layout.pos.y + 15.)
        );
    }
    p.validate().unwrap();
    p.diagrams[0].nodes[0].boundary = Some(p.diagrams[0].nodes[1].id);
    assert!(p.diagnostics().iter().any(|d| d.message.contains("순환")));
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recovery.umlproj");
    storage::save_recovery(&path, &p).unwrap();
    assert!(storage::save(&path, &p).is_err());
    assert_eq!(storage::load(&path).unwrap(), p);
    commands::move_nodes(&mut p.diagrams[0], &selected, Point::new(1., 1.));
}
#[test]
fn nested_alignment_and_distribution_translate_descendants() {
    let mut p = nested();
    let d = &mut p.diagrams[0];
    let other = Node::new(NodeKind::Package, Point::new(500., 100.));
    let selected = HashSet::from([d.nodes[0].id, other.id]);
    d.nodes.push(other);
    let offset = d.nodes[2].layout.pos.y - d.nodes[0].layout.pos.y;
    commands::align(d, &selected, 3);
    assert_eq!(d.nodes[2].layout.pos.y - d.nodes[0].layout.pos.y, offset);
    let last = Node::new(NodeKind::Package, Point::new(900., 200.));
    let selected = HashSet::from([d.nodes[0].id, d.nodes[3].id, last.id]);
    d.nodes.push(last);
    let offset = d.nodes[2].layout.pos.x - d.nodes[0].layout.pos.x;
    commands::distribute(d, &selected, true);
    assert_eq!(d.nodes[2].layout.pos.x - d.nodes[0].layout.pos.x, offset);
}
#[test]
fn auto_usecase_route_is_direct_when_clear_and_waypoints_survive_mode_changes() {
    let mut d = Diagram::new(DiagramKind::UseCase, "d");
    let a = Node::new(NodeKind::UseCase, Point::new(0., 0.));
    let b = Node::new(NodeKind::UseCase, Point::new(400., 210.));
    let mut r = Relation::new(a.id, b.id, RelationKind::Include);
    d.nodes = vec![a, b];
    assert_eq!(routing::path(&d, &r).len(), 2);
    r.waypoints = vec![Point::new(300., -60.), Point::new(390., -60.)];
    let original = r.waypoints.clone();
    for mode in [RouteMode::Auto, RouteMode::Straight, RouteMode::Orthogonal] {
        r.route_mode = mode;
        let path = routing::path(&d, &r);
        assert_eq!(r.waypoints, original);
        assert!(
            path.windows(2)
                .all(|w| (w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01)
        );
    }
}
#[test]
fn scene_cache_reuses_view_changes_and_invalidates_content_and_undo() {
    let mut p = nested();
    let mut cache = scene::SceneCache::default();
    let first = cache.get(&p.diagrams[0]);
    p.diagrams[0].view.zoom = 2.;
    assert!(Arc::ptr_eq(&first, &cache.get(&p.diagrams[0])));
    let before = p.clone();
    p.diagrams[0].nodes[2].name = "변경".into();
    let second = cache.get(&p.diagrams[0]);
    assert!(!Arc::ptr_eq(&first, &second));
    let mut history = History::default();
    history.record(before, &p);
    history.undo(&mut p);
    assert!(!Arc::ptr_eq(&second, &cache.get(&p.diagrams[0])));
}
#[test]
fn seeded_random_edits_keep_ids_references_and_undo_redo_roundtrips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("무작위.umlproj");
    let mut p = nested();
    let mut history = History::default();
    let mut seed = 0x5eed_u64;
    for _ in 0..200 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let before = p.clone();
        let d = &mut p.diagrams[0];
        let index = (seed as usize >> 8) % d.nodes.len();
        let selected = HashSet::from([d.nodes[index].id]);
        match seed % 4 {
            0 => commands::move_nodes(d, &selected, Point::new(2., -1.)),
            1 => d.nodes[index].name.push('한'),
            2 if d.nodes.len() < 40 => {
                let clip = Clipboard::copy(d, &selected);
                clip.paste(d);
            }
            _ => {
                let b = d.nodes[index].id;
                if d.nodes[index].is_classifier() {
                    d.add_relation(Relation::new(b, b, RelationKind::Association))
                        .unwrap();
                }
            }
        }
        history.record(before.clone(), &p);
        if !before.content_eq(&p) {
            let after = p.clone();
            history.undo(&mut p);
            assert!(p.content_eq(&before));
            history.redo(&mut p);
            assert!(p.content_eq(&after));
        }
        p.validate().unwrap();
    }
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
}

#[test]
fn extreme_finite_route_coordinates_are_rejected_before_rendering() {
    let mut p = Project::new("p", DiagramKind::Class, "d");
    let a = Node::new(NodeKind::Class, Point::default());
    let b = Node::new(NodeKind::Class, Point::new(400., 0.));
    let mut r = Relation::new(a.id, b.id, RelationKind::Dependency);
    r.waypoints = vec![Point::new(1e30, 0.)];
    p.diagrams[0].nodes = vec![a, b];
    p.diagrams[0].relations.push(r);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("extreme.umlproj");
    fs::write(&path, serde_json::to_vec(&p).unwrap()).unwrap();
    assert!(storage::load(&path).is_err());
    assert!(storage::save_recovery(&path, &p).is_err());
    p.diagrams[0].relations[0].waypoints.clear();
    p.diagrams[0].relations[0].bend = Some(1e30);
    assert!(p.validate_structure().is_err());
}

#[test]
fn dangling_fragment_participant_scope_is_diagnosed_losslessly() {
    let mut p = uml_studio::demo::review_project();
    let d = p
        .diagrams
        .iter_mut()
        .find(|d| d.kind == DiagramKind::Sequence)
        .unwrap();
    let fragment = &mut d.sequence.fragments[0];
    fragment.participant_start = Some(id());
    let fragment_id = fragment.id;
    assert!(
        p.diagnostics()
            .iter()
            .any(|issue| issue.element == fragment_id && issue.message.contains("참여자 범위"))
    );
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("recovery.umlproj");
    storage::save_recovery(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
}

#[test]
fn old_preferences_load_and_dark_theme_keeps_custom_text_legible() {
    let old: storage::Preferences = serde_json::from_str(r#"{"recent":[]}"#).unwrap();
    assert!(!old.dark_mode);
    let ctx = eframe::egui::Context::default();
    uml_studio::theme::apply_mode(&ctx, true);
    let _ = ctx.run(Default::default(), |ctx| {
        eframe::egui::CentralPanel::default().show(ctx, |ui| {
            let text = uml_studio::theme::ui_color(ui, uml_studio::theme::TEXT);
            assert!(text.r() > 200 && ui.visuals().panel_fill.r() < 80);
        });
    });
    uml_studio::theme::apply_mode(&ctx, false);
    assert!(!ctx.style().visuals.dark_mode);
}

#[test]
fn culled_geometry_still_participates_in_hit_testing_and_export_bounds() {
    let mut d = Diagram::new(DiagramKind::Class, "d");
    let near = Node::new(NodeKind::Class, Point::new(10., 10.));
    let far = Node::new(NodeKind::Class, Point::new(5000., 5000.));
    let far_id = far.id;
    d.nodes = vec![near, far];
    let drawing = scene::build(&d);
    assert_eq!(drawing.hit(Point::new(5010., 5010.), 6.), Some(far_id));
    assert!(drawing.bounds().unwrap().size.x > 5000.);
    let ctx = eframe::egui::Context::default();
    let output = ctx.run(
        eframe::egui::RawInput {
            screen_rect: Some(eframe::egui::Rect::from_min_size(
                eframe::egui::Pos2::ZERO,
                eframe::egui::vec2(400., 300.),
            )),
            ..Default::default()
        },
        |ctx| {
            let painter = ctx.layer_painter(eframe::egui::LayerId::background());
            scene::paint(
                &drawing,
                &painter,
                eframe::egui::Pos2::ZERO,
                Point::default(),
                1.,
            );
        },
    );
    assert!(!output.shapes.is_empty());
    for shape in output.shapes {
        assert!(shape.shape.visual_bounding_rect().min.x < 1000.);
    }
}
