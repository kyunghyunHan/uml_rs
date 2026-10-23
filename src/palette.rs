use crate::{canvas::Tool, model::*};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke};
pub fn toolbar_style(ui: &mut egui::Ui) {
    let spacing = ui.spacing_mut();
    spacing.interact_size.y = 32.;
    spacing.button_padding = egui::vec2(10., 6.);
    spacing.item_spacing = egui::vec2(8., 8.);
}

pub fn toolbar_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::from_rgb(247, 249, 252))
        .inner_margin(egui::Margin::symmetric(8, 6))
        .stroke(Stroke::new(1_f32, Color32::from_rgb(221, 226, 234)))
}

pub fn relation_label(diagram: DiagramKind, kind: RelationKind) -> String {
    let targets = match (diagram, kind) {
        (DiagramKind::UseCase, RelationKind::Association) => "액터 ↔ 유스케이스",
        (_, RelationKind::Include | RelationKind::Extend) => "유스케이스 → 유스케이스",
        (DiagramKind::UseCase, RelationKind::Generalization) => "액터끼리 또는 유스케이스끼리",
        (_, RelationKind::Generalization) => "클래스끼리 또는 인터페이스끼리 · 하위 → 상위",
        (_, RelationKind::Realization) => "구현 클래스 → 인터페이스",
        (_, RelationKind::Aggregation | RelationKind::Composition) => "클래스 요소 · 전체 → 부분",
        (_, RelationKind::Comment) => "노트 ↔ 요소",
        _ => "클래스·인터페이스·열거형 → 클래스·인터페이스·열거형",
    };
    format!("{}: {targets}", kind.label())
}

pub fn node_tool(ui: &mut egui::Ui, tool: &mut Tool, kind: NodeKind) {
    let text = kind.label();
    let width = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.into(), FontId::proportional(13.), Color32::BLACK)
            .size()
            .x
    }) + 48.;
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 32.), Sense::click());
    let selected = *tool == Tool::Add(kind);
    let color = if selected {
        Color32::from_rgb(45, 83, 183)
    } else {
        Color32::from_rgb(65, 73, 89)
    };
    ui.painter().rect_filled(
        rect,
        4.,
        if selected {
            Color32::from_rgb(222, 232, 255)
        } else if response.hovered() {
            Color32::from_gray(234)
        } else {
            Color32::TRANSPARENT
        },
    );
    let center = rect.min + egui::vec2(17., 16.);
    let stroke = Stroke::new(1.5_f32, color);
    match kind {
        NodeKind::Actor => {
            ui.painter()
                .circle_stroke(center + egui::vec2(0., -7.), 3., stroke);
            for (a, b) in [
                ([0., -4.], [0., 4.]),
                ([-6., -1.], [6., -1.]),
                ([0., 4.], [-5., 10.]),
                ([0., 4.], [5., 10.]),
            ] {
                ui.painter().line_segment(
                    [
                        center + egui::vec2(a[0], a[1]),
                        center + egui::vec2(b[0], b[1]),
                    ],
                    stroke,
                );
            }
        }
        NodeKind::UseCase => {
            ui.painter().add(egui::epaint::EllipseShape {
                center,
                radius: egui::vec2(11., 7.),
                fill: Color32::TRANSPARENT,
                stroke,
            });
        }
        _ => {
            let r = egui::Rect::from_center_size(center, egui::vec2(20., 22.));
            ui.painter()
                .rect_stroke(r, 0., stroke, egui::StrokeKind::Inside);
            if matches!(
                kind,
                NodeKind::Class
                    | NodeKind::Interface
                    | NodeKind::AbstractClass
                    | NodeKind::Enumeration
            ) {
                for y in [-3., 4.] {
                    ui.painter().line_segment(
                        [center + egui::vec2(-10., y), center + egui::vec2(10., y)],
                        stroke,
                    );
                }
            }
        }
    }
    ui.painter().text(
        rect.min + egui::vec2(34., 16.),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.),
        color,
    );
    if response.clicked() {
        *tool = Tool::Add(kind);
        ui.close();
    }
    response.on_hover_text(format!(
        "{} 추가: 선택 후 캔버스 클릭 · Esc 취소",
        kind.label()
    ));
}

pub fn sequence_button(
    ui: &mut egui::Ui,
    label: &str,
    kind: usize,
    enabled: bool,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled,
        egui::Button::new(format!("     {label}")).min_size(egui::vec2(145., 32.)),
    );
    let c = response.rect.min + egui::vec2(16., 16.);
    let color = if enabled {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    let stroke = Stroke::new(1.4_f32, color);
    match kind {
        0 => {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(c + egui::vec2(0., -4.), egui::vec2(18., 12.)),
                0.,
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter()
                .line_segment([c + egui::vec2(0., 2.), c + egui::vec2(0., 11.)], stroke);
        }
        1 => {
            ui.painter()
                .arrow(c - egui::vec2(10., 0.), egui::vec2(20., 0.), stroke);
        }
        2 => {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(7., 22.)),
                0.,
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        _ => {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(22., 20.)),
                0.,
                stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().line_segment(
                [c + egui::vec2(-11., -3.), c + egui::vec2(11., -3.)],
                stroke,
            );
        }
    }
    response.on_hover_text(format!("{label}를 추가한 뒤 속성 패널에서 편집하세요"))
}
