use crate::{canvas::Tool, model::*};
use eframe::egui::{self, Align2, Color32, FontId, Sense, Stroke};
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
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(width, crate::theme::CONTROL_HEIGHT),
        Sense::click(),
    );
    let selected = *tool == Tool::Add(kind);
    let color = if selected {
        crate::theme::BLUE_TEXT
    } else {
        crate::theme::TEXT
    };
    ui.painter().rect_filled(
        rect,
        4.,
        if selected {
            crate::theme::BLUE_LIGHT
        } else if response.hovered() || response.has_focus() {
            crate::theme::HOVER
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
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            text,
        )
    });
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
    let response = ui
        .add_enabled_ui(enabled, |ui| {
            let (rect, response) = ui.allocate_exact_size(
                egui::vec2(160., crate::theme::CONTROL_HEIGHT),
                Sense::click(),
            );
            let visuals = ui.style().interact(&response);
            ui.painter().rect(
                rect,
                4.,
                visuals.weak_bg_fill,
                visuals.bg_stroke,
                egui::StrokeKind::Inside,
            );
            ui.painter().text(
                rect.min + egui::vec2(36., 16.),
                Align2::LEFT_CENTER,
                label,
                FontId::proportional(13.),
                visuals.text_color(),
            );
            response
        })
        .inner;
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
    response.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, enabled, label));
    response.on_hover_text(format!("{label}를 추가한 뒤 속성 패널에서 편집하세요"))
}

#[derive(Clone)]
pub struct RelationChoice {
    pub kind: RelationKind,
    pub preset: NavigationPreset,
    pub label: String,
}
pub fn relation_choices(diagram: DiagramKind) -> Vec<RelationChoice> {
    let mut choices = vec![];
    for &kind in RelationKind::allowed(diagram) {
        if diagram == DiagramKind::Class && kind == RelationKind::Association {
            for preset in NavigationPreset::ALL {
                choices.push(RelationChoice {
                    kind,
                    preset,
                    label: format!(
                        "{} · {}",
                        if preset == NavigationPreset::Unspecified {
                            "일반 연관"
                        } else {
                            "방향 연관"
                        },
                        preset.label()
                    ),
                });
            }
        } else {
            choices.push(RelationChoice {
                kind,
                preset: NavigationPreset::Unspecified,
                label: relation_label(diagram, kind),
            });
        }
    }
    choices
}
pub fn relation_option(
    ui: &mut egui::Ui,
    label: &str,
    kind: RelationKind,
    preset: NavigationPreset,
    selected: bool,
) -> egui::Response {
    let (source, target) = preset.ends();
    relation_option_ends(ui, label, kind, source, target, selected)
}
pub fn relation_option_ends(
    ui: &mut egui::Ui,
    label: &str,
    kind: RelationKind,
    source: Navigability,
    target: Navigability,
    selected: bool,
) -> egui::Response {
    let width = (crate::model::text_width(label, 13.) + 104.).clamp(260., 580.);
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(
            width.min((ui.ctx().content_rect().width() - 32.).max(260.)),
            crate::theme::CONTROL_HEIGHT,
        ),
        Sense::click(),
    );
    ui.painter().rect_filled(
        rect,
        4.,
        if selected {
            crate::theme::BLUE_LIGHT
        } else if response.hovered() || response.has_focus() {
            crate::theme::HOVER
        } else {
            Color32::TRANSPARENT
        },
    );
    let preview = crate::scene::relation_preview(kind, source, target);
    crate::scene::paint(
        &preview,
        ui.painter(),
        rect.min + egui::vec2(10., 16.),
        Point::default(),
        0.72,
    );
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        (rect.width() - 90.).max(1.),
        egui::TextStyle::Button,
    );
    ui.painter().galley(
        rect.min + egui::vec2(86., (32. - galley.size().y) / 2.),
        galley,
        if selected {
            crate::theme::BLUE_TEXT
        } else {
            crate::theme::TEXT
        },
    );
    response.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            selected,
            label,
        )
    });
    response.on_hover_text(label)
}
