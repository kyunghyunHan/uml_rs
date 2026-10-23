use eframe::egui::{self, Event, Key, Modifiers, Rect};
use uml_studio::{components, model::DiagramKind, theme};

fn tab_frame(
    ctx: &egui::Context,
    events: Vec<Event>,
) -> (egui::FullOutput, components::TabResponse) {
    let mut tab = None;
    let time = ctx.input(|i| i.time) + 0.1;
    let output = ctx.run(egui::RawInput {
        screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400., 180.))),
        events, time: Some(time), ..Default::default()
    }, |ctx| {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                tab = Some(components::diagram_tab(ui, egui::Id::new("test-tab"), DiagramKind::Class, "Very long diagram name that must be truncated without moving the close button", false));
            });
        });
    });
    (output, tab.unwrap())
}

fn cross_count(output: &egui::FullOutput, rect: Rect) -> usize {
    output
        .shapes
        .iter()
        .filter(|shape| {
            matches!(&shape.shape,
                egui::Shape::LineSegment { points, .. } if points.iter().all(|p| rect.contains(*p))
            )
        })
        .count()
}

#[test]
fn inactive_tab_reserves_its_close_area_and_shows_cross_on_hover_and_keyboard_focus() {
    let ctx = egui::Context::default();
    theme::apply(&ctx);
    tab_frame(&ctx, vec![]);
    let (output, original) = tab_frame(&ctx, vec![]);
    assert!(!original.select.rect.intersects(original.close.rect));
    assert!(original.close.rect.width() >= 24. && original.close.rect.height() >= 24.);
    assert_eq!(cross_count(&output, original.close.rect), 0);
    let (output, hovered) = tab_frame(
        &ctx,
        vec![Event::PointerMoved(original.close.rect.center())],
    );
    assert_eq!(original.select.rect, hovered.select.rect);
    assert_eq!(original.close.rect, hovered.close.rect);
    assert_eq!(cross_count(&output, original.close.rect), 2);
    // Text is laid out to the reserved name width and never touches the close hit area.
    for shape in &output.shapes {
        if let egui::Shape::Text(text) = &shape.shape {
            assert!(text.pos.x + text.galley.size().x < original.close.rect.left());
            assert!(text.galley.elided);
        }
    }
    ctx.memory_mut(|m| m.request_focus(egui::Id::new("test-tab").with("close")));
    let (output, focused) = tab_frame(&ctx, vec![Event::PointerGone]);
    assert!(focused.close.has_focus());
    assert_eq!(cross_count(&output, original.close.rect), 2);
    let (_, activated) = tab_frame(
        &ctx,
        vec![Event::Key {
            key: Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: Modifiers::NONE,
        }],
    );
    assert!(activated.close.clicked());
    assert!(!activated.select.clicked());
}
