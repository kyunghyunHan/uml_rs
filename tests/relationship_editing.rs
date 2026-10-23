use std::collections::HashSet;
use uml_studio::{
    canvas, commands, export,
    model::*,
    palette, routing,
    scene::{self, Primitive},
    storage, validation,
};

fn node(name: &str, x: f32, y: f32) -> Node {
    let mut n = Node::new(NodeKind::Class, Point::new(x, y));
    n.name = name.into();
    n.sizing = SizeMode::Manual;
    n.layout.size = Point::new(180., 100.);
    n
}
fn diagram(x: f32, y: f32) -> (Diagram, Relation) {
    let mut d = Diagram::new(DiagramKind::Class, "relations");
    d.nodes = vec![node("Source", 40., 80.), node("Target", x, y)];
    let r = Relation::new(d.nodes[0].id, d.nodes[1].id, RelationKind::Association);
    (d, r)
}
fn orthogonal(points: &[Point]) -> bool {
    points
        .windows(2)
        .all(|w| w[0] != w[1] && ((w[0].x - w[1].x).abs() < 0.01 || (w[0].y - w[1].y).abs() < 0.01))
}
fn on_border(p: Point, r: Layout) -> bool {
    let inside = p.x >= r.pos.x - 0.01
        && p.x <= r.pos.x + r.size.x + 0.01
        && p.y >= r.pos.y - 0.01
        && p.y <= r.pos.y + r.size.y + 0.01;
    inside
        && ((p.x - r.pos.x).abs() < 0.01
            || (p.x - r.pos.x - r.size.x).abs() < 0.01
            || (p.y - r.pos.y).abs() < 0.01
            || (p.y - r.pos.y - r.size.y).abs() < 0.01)
}

#[test]
fn v1_and_v2_navigation_migrate_without_turning_false_into_non_navigable() {
    let dir = tempfile::tempdir().unwrap();
    for version in [1, 2] {
        for old in [false, true] {
            let (d, r) = diagram(400., 80.);
            let mut p = Project::new("legacy", DiagramKind::Class, "old");
            p.diagrams[0] = d;
            p.diagrams[0].relations.push(r.clone());
            let mut value = serde_json::to_value(p).unwrap();
            value["format_version"] = version.into();
            let fields = value["diagrams"][0]["relations"][0]
                .as_object_mut()
                .unwrap();
            for key in [
                "source_navigation",
                "target_navigation",
                "route_mode",
                "source_port",
                "target_port",
            ] {
                fields.remove(key);
            }
            fields.insert("navigable".into(), old.into());
            let path = dir.path().join("legacy.umlproj");
            std::fs::write(&path, serde_json::to_vec(&value).unwrap()).unwrap();
            let p = storage::load(&path).unwrap();
            let loaded = &p.diagrams[0].relations[0];
            assert_eq!(p.format_version, 3);
            assert_eq!(loaded.id, r.id);
            assert_eq!(loaded.source_navigation, Navigability::Unspecified);
            assert_eq!(
                loaded.target_navigation,
                if old {
                    Navigability::Navigable
                } else {
                    Navigability::Unspecified
                }
            );
            storage::save(&path, &p).unwrap();
            assert_eq!(storage::load(&path).unwrap(), p);
            assert!(
                !std::fs::read_to_string(path)
                    .unwrap()
                    .contains("\"navigable\"")
            );
        }
    }
}

#[test]
fn all_navigability_states_are_independent_of_diamonds_and_plain_associations_stay_plain() {
    for kind in [
        RelationKind::Association,
        RelationKind::Aggregation,
        RelationKind::Composition,
    ] {
        for source in Navigability::ALL {
            for target in Navigability::ALL {
                let (mut d, mut r) = diagram(500., 80.);
                r.kind = kind;
                r.source_navigation = source;
                r.target_navigation = target;
                d.add_relation(r.clone()).unwrap();
                let sc = scene::build(&d);
                let paths: Vec<_> = sc
                    .items
                    .iter()
                    .filter(|i| i.owner == r.id)
                    .filter_map(|i| {
                        if let Primitive::Path {
                            points,
                            closed,
                            fill,
                            ..
                        } = &i.primitive
                        {
                            Some((points, *closed, *fill))
                        } else {
                            None
                        }
                    })
                    .collect();
                let extra = |n| match n {
                    Navigability::Unspecified => 0,
                    Navigability::Navigable => 1,
                    Navigability::NonNavigable => 2,
                };
                assert_eq!(
                    paths.len(),
                    1 + usize::from(kind != RelationKind::Association)
                        + extra(source)
                        + extra(target)
                );
                if kind != RelationKind::Association {
                    assert!(paths[1].1 && paths[1].0.len() == 4);
                    assert_eq!(paths[1].0[0], routing::path(&d, &r)[0]);
                    assert_eq!(
                        paths[1].2,
                        if kind == RelationKind::Composition {
                            2
                        } else {
                            1
                        }
                    );
                }
                let pts = routing::path(&d, &r);
                for (state, tip, previous) in [
                    (source, pts[0], pts[1]),
                    (target, *pts.last().unwrap(), pts[pts.len() - 2]),
                ] {
                    if state == Navigability::Navigable {
                        let gap = if tip == pts[0] && kind != RelationKind::Association {
                            26.
                        } else {
                            0.
                        };
                        let len = (previous.x - tip.x).hypot(previous.y - tip.y);
                        let expected = Point::new(
                            tip.x + (previous.x - tip.x) / len * gap,
                            tip.y + (previous.y - tip.y) / len * gap,
                        );
                        assert!(
                            paths
                                .iter()
                                .any(|(p, closed, _)| !closed && p.len() == 3 && p[1] == expected)
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn automatic_straight_and_orthogonal_routes_handle_geometry_and_obstacles_without_moving_nodes() {
    for (x, y) in [
        (500., 80.),
        (40., 400.),
        (500., 350.),
        (160., 120.),
        (40., 80.),
    ] {
        let (d, mut r) = diagram(x, y);
        let nodes = d.nodes.clone();
        for mode in RouteMode::ALL {
            r.route_mode = mode;
            let pts = routing::path(&d, &r);
            assert!(pts.len() >= 2 && pts.iter().all(|p| p.x.is_finite() && p.y.is_finite()));
            assert!(on_border(pts[0], d.nodes[0].layout));
            assert!(on_border(*pts.last().unwrap(), d.nodes[1].layout));
            if mode == RouteMode::Orthogonal {
                assert!(orthogonal(&pts));
            }
            if (x, y) == (500., 80.) || (x, y) == (40., 400.) {
                assert_eq!(pts.len(), 2, "unnecessary elbows: {mode:?}");
            }
        }
        assert_eq!(d.nodes, nodes);
    }
    let (mut d, mut r) = diagram(700., 80.);
    d.nodes.push(node("Obstacle", 350., 50.));
    for mode in [RouteMode::Auto, RouteMode::Orthogonal] {
        r.route_mode = mode;
        let pts = routing::path(&d, &r);
        assert!(orthogonal(&pts));
        assert!(
            pts.windows(2).all(|w| d
                .nodes
                .iter()
                .all(|n| !routing::segment_hits(w[0], w[1], n.layout))),
            "path crossed a classifier: {pts:?}"
        );
    }
    r.route_mode = RouteMode::Straight;
    assert_eq!(routing::path(&d, &r).len(), 2);
    let clean = routing::simplify(vec![
        Point::new(0., 0.),
        Point::new(0., 0.),
        Point::new(20., 0.),
        Point::new(40., 0.),
        Point::new(40., 20.),
    ]);
    assert_eq!(
        clean,
        vec![
            Point::new(0., 0.),
            Point::new(40., 0.),
            Point::new(40., 20.)
        ]
    );
}

#[test]
fn manual_routes_reverse_with_all_end_metadata_and_survive_modes_history_and_storage() {
    let (mut d, mut r) = diagram(650., 300.);
    r.source_role = "owner".into();
    r.target_role = "parts".into();
    r.source_multiplicity = "1".into();
    r.target_multiplicity = "0..*".into();
    r.source_navigation = Navigability::NonNavigable;
    r.target_navigation = Navigability::Navigable;
    r.source_port = ConnectionPoint::centered(PortSide::Right);
    r.target_port = ConnectionPoint::centered(PortSide::Left);
    r.waypoints = vec![
        Point::new(320., 130.),
        Point::new(320., 350.),
        Point::new(550., 350.),
    ];
    let authored = r.waypoints.clone();
    d.add_relation(r.clone()).unwrap();
    for mode in RouteMode::ALL {
        r.route_mode = mode;
        let _ = routing::path(&d, &r);
        assert_eq!(r.waypoints, authored);
    }
    r.route_mode = RouteMode::Auto;
    let original = routing::path(&d, &r);
    let reversed = commands::reversed_relation(&d, &r);
    assert_eq!((reversed.source, reversed.target), (r.target, r.source));
    assert_eq!(
        (&reversed.source_role, &reversed.target_role),
        (&r.target_role, &r.source_role)
    );
    assert_eq!(
        (&reversed.source_multiplicity, &reversed.target_multiplicity),
        (&r.target_multiplicity, &r.source_multiplicity)
    );
    assert_eq!(
        (reversed.source_navigation, reversed.target_navigation),
        (r.target_navigation, r.source_navigation)
    );
    assert_eq!(
        (reversed.source_port, reversed.target_port),
        (r.target_port, r.source_port)
    );
    let mut expected = original.clone();
    expected.reverse();
    assert_eq!(routing::path(&d, &reversed), expected);
    let mut bent = r.clone();
    bent.waypoints.clear();
    bent.bend = Some(400.);
    let mut expected = routing::path(&d, &bent);
    expected.reverse();
    assert_eq!(
        routing::path(&d, &commands::reversed_relation(&d, &bent)),
        expected
    );
    let mut p = Project::new("history", DiagramKind::Class, "test");
    p.diagrams[0] = d;
    let before = p.clone();
    p.diagrams[0].update_relation(reversed).unwrap();
    let after = p.clone();
    let mut history = commands::History::default();
    history.record(before.clone(), &p);
    history.undo(&mut p);
    assert_eq!(p, before);
    history.redo(&mut p);
    assert_eq!(p, after);
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("v3.umlproj");
    storage::save(&path, &p).unwrap();
    assert_eq!(storage::load(&path).unwrap(), p);
    for mode in RouteMode::ALL {
        p.diagrams[0].relations[0].route_mode = mode;
        storage::save(&path, &p).unwrap();
        assert_eq!(storage::load(&path).unwrap(), p);
    }
}

#[test]
fn multiplicity_syntax_ranges_and_composition_owner_are_validated_on_create_and_update() {
    for text in ["", "0", "1", "*", "0..1", "1..*", "2..5", "1, 3..5"] {
        assert!(validation::multiplicity(text).is_ok(), "{text}");
    }
    for text in [
        "-1",
        "1.5",
        "2..1",
        "*..5",
        "1..",
        "1,",
        "a",
        "1...5",
        "999999999999999999999999999",
    ] {
        assert!(validation::multiplicity(text).is_err(), "{text}");
    }
    let (mut d, mut r) = diagram(400., 80.);
    r.kind = RelationKind::Composition;
    for text in ["2", "*", "0..2", "1..*"] {
        r.source_multiplicity = text.into();
        assert!(d.add_relation(r.clone()).is_err());
    }
    r.source_multiplicity = "0..1".into();
    d.add_relation(r.clone()).unwrap();
    let before = d.clone();
    r.source_multiplicity = "2".into();
    assert!(d.update_relation(r.clone()).is_err());
    assert_eq!(d, before);
    let mut p = Project::new("bad", DiagramKind::Class, "test");
    p.diagrams[0] = d;
    p.diagrams[0].relations[0] = r;
    assert!(
        p.diagnostics()
            .iter()
            .any(|d| d.message.contains("전체(시작)"))
    );
}

#[test]
fn allowed_kinds_and_class_start_endpoint_candidates_share_validation() {
    for diagram in [
        DiagramKind::Class,
        DiagramKind::UseCase,
        DiagramKind::Sequence,
    ] {
        let choices = palette::relation_choices(diagram);
        assert!(
            choices
                .iter()
                .all(|c| RelationKind::allowed(diagram).contains(&c.kind))
        );
    }
    let (mut d, mut r) = diagram(400., 80.);
    d.nodes[1].kind = NodeKind::Interface;
    r.kind = RelationKind::Generalization;
    assert!(validation::endpoint_candidate(&d, &r, d.nodes[1].id, false).is_err());
    r.kind = RelationKind::Realization;
    assert!(validation::endpoint_candidate(&d, &r, d.nodes[1].id, false).is_ok());
    assert!(validation::connection_start(&d, d.nodes[1].id, RelationKind::Realization).is_err());
    d.nodes[0].kind = NodeKind::Note;
    assert!(validation::connection_start(&d, d.nodes[0].id, RelationKind::Association).is_err());
}

#[test]
fn ports_remain_on_borders_after_move_resize_and_fit_selection_does_not_edit_content() {
    let (mut d, mut r) = diagram(400., 300.);
    for side in PortSide::ALL {
        r.source_port = ConnectionPoint::centered(side);
        for offset in [0., 0.3, 0.5, 1.] {
            r.source_port.offset = offset;
            let pts = routing::path(&d, &r);
            assert!(on_border(pts[0], d.nodes[0].layout));
            d.nodes[0].layout.pos.x += 5.;
            d.nodes[0].layout.size.y += 10.;
            let pts = routing::path(&d, &r);
            assert!(on_border(pts[0], d.nodes[0].layout));
        }
    }
    d.add_relation(r.clone()).unwrap();
    let nodes = d.nodes.clone();
    let relations = d.relations.clone();
    assert!(canvas::fit_selection(
        &mut d,
        eframe::egui::vec2(800., 600.),
        &HashSet::from([r.id]),
        None
    ));
    assert_eq!(d.nodes, nodes);
    assert_eq!(d.relations, relations);
    assert!(canvas::fit_selection(
        &mut d,
        eframe::egui::vec2(800., 600.),
        &HashSet::new(),
        Some(Layout::new(20., 20., 100., 80.))
    ));
    let selected = HashSet::from([d.nodes[0].id, d.nodes[1].id]);
    commands::align(&mut d, &selected, 4);
    assert!((d.nodes[0].layout.center().x - d.nodes[1].layout.center().x).abs() < 0.01);
    commands::align(&mut d, &selected, 5);
    assert!((d.nodes[0].layout.center().y - d.nodes[1].layout.center().y).abs() < 0.01);
}

#[test]
fn self_loops_parallel_relations_and_other_endpoint_snap_remain_distinct_and_attached() {
    let (mut d, mut r) = diagram(500., 80.);
    r.target = r.source;
    for mode in RouteMode::ALL {
        for source in PortSide::ALL {
            for target in PortSide::ALL {
                r.route_mode = mode;
                r.source_port = ConnectionPoint::centered(source);
                r.target_port = ConnectionPoint::centered(target);
                let points = routing::path(&d, &r);
                assert!(
                    points.len() >= 4 && orthogonal(&points),
                    "invalid loop: {source:?} {target:?} {points:?}"
                );
                assert!(
                    points.windows(2).all(|w| !routing::segment_hits(
                        w[0],
                        w[1],
                        d.nodes[0].layout
                    )),
                    "loop crossed its owner: {source:?} {target:?} {points:?}"
                );
            }
        }
    }
    let first = Relation::new(d.nodes[0].id, d.nodes[1].id, RelationKind::Association);
    let mut second = first.clone();
    second.id = id();
    second.apply_navigation(NavigationPreset::Both);
    d.relations = vec![first, second];
    assert_ne!(
        routing::path(&d, &d.relations[0]),
        routing::path(&d, &d.relations[1])
    );
    d.nodes
        .extend([node("C", 80., -160.), node("D", 80., -400.)]);
    d.relations.push(Relation::new(
        d.nodes[2].id,
        d.nodes[3].id,
        RelationKind::Association,
    ));
    let (port, guides) = routing::snap_port(&d, &d.relations[0], true, Point::new(173., 80.), 7.);
    assert_eq!(port.side, PortSide::Top);
    let point = routing::port_position(&d.nodes[0], port, Point::default());
    assert_eq!(point.x, 170.);
    assert!(!guides.is_empty());
    assert!(guides.iter().all(|(a, b)| a.x == b.x));
}

#[test]
fn labels_avoid_nodes_and_each_other_and_classifier_typography_survives() {
    let (mut d, mut r) = diagram(500., 80.);
    r.name = "ownership".into();
    r.source_role = "owner".into();
    r.source_multiplicity = "1".into();
    r.target_role = "items".into();
    r.target_multiplicity = "0..*".into();
    r.apply_navigation(NavigationPreset::Both);
    d.nodes[0].attributes.push(Member {
        name: "count".into(),
        type_name: "int".into(),
        is_static: true,
        ..Default::default()
    });
    d.nodes[1].kind = NodeKind::Interface;
    d.nodes[1].methods.push(Member {
        name: "run".into(),
        type_name: "void".into(),
        is_abstract: true,
        ..Default::default()
    });
    d.add_relation(r.clone()).unwrap();
    let sc = scene::build(&d);
    let labels: Vec<_> = sc
        .items
        .iter()
        .filter(|i| i.owner == r.id)
        .filter_map(|i| {
            if let Primitive::Text { .. } = &i.primitive {
                Some(scene::primitive_bounds(&i.primitive))
            } else {
                None
            }
        })
        .collect();
    assert_eq!(labels.len(), 3);
    let intersects = |a: Layout, b: Layout| {
        a.pos.x < b.pos.x + b.size.x
            && a.pos.x + a.size.x > b.pos.x
            && a.pos.y < b.pos.y + b.size.y
            && a.pos.y + a.size.y > b.pos.y
    };
    for (index, label) in labels.iter().enumerate() {
        assert!(d.nodes.iter().all(|n| !intersects(*label, n.layout)));
        assert!(
            labels
                .iter()
                .skip(index + 1)
                .all(|other| !intersects(*label, *other))
        );
    }
    assert!(sc.items.iter().any(
        |i| matches!(&i.primitive,Primitive::Text{text,underline:true,..} if text.contains("count"))
    ));
    assert!(sc.items.iter().any(
        |i| matches!(&i.primitive,Primitive::Text{text,italic:true,..} if text.contains("run"))
    ));
    assert!(
        sc.items
            .iter()
            .any(|i| matches!(&i.primitive,Primitive::Text{text,..} if text.contains("interface")))
    );
}

#[test]
fn arrowheads_follow_the_first_and_last_segments_after_routing_and_reversal() {
    for mode in RouteMode::ALL {
        for kind in [
            RelationKind::Association,
            RelationKind::Generalization,
            RelationKind::Realization,
            RelationKind::Dependency,
            RelationKind::Aggregation,
            RelationKind::Composition,
        ] {
            let (mut d, mut r) = diagram(500., 340.);
            r.kind = kind;
            r.route_mode = mode;
            r.apply_navigation(NavigationPreset::Both);
            if kind == RelationKind::Realization {
                d.nodes[1].kind = NodeKind::Interface;
            }
            r.source_port = ConnectionPoint::centered(PortSide::Right);
            r.target_port = ConnectionPoint::centered(PortSide::Top);
            d.add_relation(r.clone()).unwrap();
            for reversed in [false, true] {
                let r = if reversed {
                    commands::reversed_relation(&d, &r)
                } else {
                    r.clone()
                };
                // Geometry remains inspectable even when semantic validation forbids reversing a realization.
                let mut diagram = d.clone();
                diagram.relations[0] = r.clone();
                let pts = routing::path(&diagram, &r);
                let sc = scene::build(&diagram);
                let mut expected = vec![(*pts.last().unwrap(), pts[pts.len() - 2])];
                if kind.has_navigation() {
                    let gap =
                        if matches!(kind, RelationKind::Aggregation | RelationKind::Composition) {
                            26.
                        } else {
                            0.
                        };
                    let a = pts[0];
                    let b = pts[1];
                    let len = (b.x - a.x).hypot(b.y - a.y);
                    expected.push((
                        Point::new(a.x + (b.x - a.x) / len * gap, a.y + (b.y - a.y) / len * gap),
                        b,
                    ));
                }
                for (tip, previous) in expected {
                    let arrow = sc
                        .items
                        .iter()
                        .filter(|i| i.owner == r.id)
                        .find_map(|i| match &i.primitive {
                            Primitive::Path { points, .. }
                                if points.len() == 3
                                    && (points[1].x - tip.x).abs() < 0.01
                                    && (points[1].y - tip.y).abs() < 0.01 =>
                            {
                                Some(points)
                            }
                            _ => None,
                        })
                        .unwrap();
                    let back = Point::new(
                        (arrow[0].x + arrow[2].x) / 2.,
                        (arrow[0].y + arrow[2].y) / 2.,
                    );
                    let u = Point::new(tip.x - previous.x, tip.y - previous.y);
                    let v = Point::new(tip.x - back.x, tip.y - back.y);
                    assert!(u.x * v.x + u.y * v.y > 0.);
                    assert!((u.x * v.y - u.y * v.x).abs() < 0.1);
                }
            }
        }
    }
}

#[test]
fn scene_canvas_and_svg_use_identical_decoration_points_and_png_matches_svg_rendering() {
    use eframe::egui;
    let dir = tempfile::tempdir().unwrap();
    for mode in RouteMode::ALL {
        for kind in [
            RelationKind::Association,
            RelationKind::Composition,
            RelationKind::Generalization,
            RelationKind::Realization,
        ] {
            let (mut d, mut r) = diagram(320., 220.);
            r.kind = kind;
            r.apply_navigation(NavigationPreset::Both);
            r.route_mode = mode;
            if kind == RelationKind::Realization {
                d.nodes[1].kind = NodeKind::Interface;
            }
            d.add_relation(r.clone()).unwrap();
            let sc = scene::build(&d);
            let opts = scene::ExportOptions::default();
            let (svg, _, _) = scene::svg(&sc, &opts).unwrap();
            for item in sc.items.iter().filter(|i| i.owner == r.id) {
                if let Primitive::Path { points, .. } = &item.primitive {
                    let coords = points
                        .iter()
                        .map(|p| format!("{},{}", p.x, p.y))
                        .collect::<Vec<_>>()
                        .join(" ");
                    assert!(svg.contains(&format!("points=\"{coords}\"")));
                }
            }
            for zoom in [0.5, 1., 1.8] {
                let ctx = egui::Context::default();
                let origin = egui::pos2(10., 20.);
                let pan = Point::new(14., 18.);
                let output = ctx.run(egui::RawInput::default(), |ctx| {
                    egui::CentralPanel::default()
                        .show(ctx, |ui| scene::paint(&sc, ui.painter(), origin, pan, zoom));
                });
                for item in sc.items.iter().filter(|i| i.owner == r.id) {
                    if let Primitive::Path {
                        points,
                        closed: true,
                        ..
                    } = &item.primitive
                    {
                        let expected: Vec<_> = points
                            .iter()
                            .map(|p| origin + egui::vec2(p.x * zoom + pan.x, p.y * zoom + pan.y))
                            .collect();
                        assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::Path(path) if path.closed && path.points==expected)));
                    }
                    if let Primitive::Path {
                        points,
                        closed: false,
                        dash: false,
                        ..
                    } = &item.primitive
                    {
                        for pair in points.windows(2) {
                            let expected = [pair[0], pair[1]].map(|p| {
                                origin + egui::vec2(p.x * zoom + pan.x, p.y * zoom + pan.y)
                            });
                            assert!(output.shapes.iter().any(|s|matches!(&s.shape,egui::Shape::LineSegment{points,..} if *points==expected)));
                        }
                    }
                }
            }
            let png_path = dir.path().join("relation.png");
            let svg_path = dir.path().join("relation.svg");
            let (w, h) = export::png(&sc, &png_path, &opts).unwrap();
            export::svg(&sc, &svg_path, &opts).unwrap();
            let actual = resvg::tiny_skia::Pixmap::load_png(&png_path).unwrap();
            let tree = resvg::usvg::Tree::from_data(
                &std::fs::read(svg_path).unwrap(),
                &resvg::usvg::Options::default(),
            )
            .unwrap();
            let mut expected = resvg::tiny_skia::Pixmap::new(w, h).unwrap();
            resvg::render(
                &tree,
                resvg::tiny_skia::Transform::identity(),
                &mut expected.as_mut(),
            );
            let differing = actual
                .data()
                .iter()
                .zip(expected.data())
                .filter(|(a, b)| a.abs_diff(**b) > 8)
                .count();
            assert!(
                (differing as f32) / (actual.data().len() as f32) < 0.005,
                "PNG/SVG raster mismatch"
            );
        }
    }
}
