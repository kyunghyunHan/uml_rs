//! Small editor components; all interaction regions have stable geometry.
use crate::{model::DiagramKind, theme};
use eframe::egui::{self, Color32, Rect, Response, Sense, Stroke};

pub fn diagram_icon(p: &egui::Painter, center: egui::Pos2, kind: DiagramKind, color: Color32) {
    let stroke = Stroke::new(1.3_f32, color);
    match kind {
        DiagramKind::UseCase => {
            p.add(egui::epaint::EllipseShape {
                center,
                radius: egui::vec2(7., 4.5),
                fill: Color32::TRANSPARENT,
                stroke,
            });
        }
        DiagramKind::Class => {
            let rect = Rect::from_center_size(center, egui::vec2(13., 14.));
            p.rect_stroke(rect, 1., stroke, egui::StrokeKind::Inside);
            for y in [-2., 2.] {
                p.line_segment(
                    [center + egui::vec2(-6., y), center + egui::vec2(6., y)],
                    stroke,
                );
            }
        }
        DiagramKind::Sequence => {
            for x in [-5., 5.] {
                p.line_segment(
                    [center + egui::vec2(x, -7.), center + egui::vec2(x, 7.)],
                    stroke,
                );
            }
            p.arrow(center + egui::vec2(-5., -2.), egui::vec2(10., 0.), stroke);
            p.arrow(center + egui::vec2(5., 4.), egui::vec2(-10., 0.), stroke);
        }
    }
}

fn text(ui: &egui::Ui, rect: Rect, label: &str, color: Color32) {
    let galley = egui::WidgetText::from(label).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.width().max(1.),
        egui::TextStyle::Button,
    );
    ui.painter().galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.),
        galley,
        color,
    );
}

pub struct TabResponse {
    pub select: Response,
    pub close: Response,
}

pub fn diagram_tab(
    ui: &mut egui::Ui,
    id: egui::Id,
    kind: DiagramKind,
    name: &str,
    active: bool,
) -> TabResponse {
    let (_, rect) = ui.allocate_space(egui::vec2(188., 34.));
    let close_rect = Rect::from_center_size(
        egui::pos2(rect.right() - 17., rect.center().y),
        egui::Vec2::splat(theme::ICON_SIZE),
    );
    let name_rect = Rect::from_min_max(rect.min, egui::pos2(close_rect.left() - 2., rect.bottom()));
    // These hit regions never overlap: closing a background tab cannot select it.
    let select = ui
        .interact(name_rect, id.with("select"), Sense::click())
        .on_hover_text(name);
    let close = ui
        .interact(close_rect, id.with("close"), Sense::click())
        .on_hover_text("탭 닫기 · 다이어그램은 유지됩니다");
    let hover = ui.rect_contains_pointer(rect);
    let focused = select.has_focus() || close.has_focus();
    ui.painter().rect_filled(
        rect,
        4.,
        if active {
            Color32::WHITE
        } else if hover || focused {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        },
    );
    if active {
        ui.painter().line_segment(
            [rect.left_bottom(), rect.right_bottom()],
            Stroke::new(2_f32, theme::BLUE),
        );
    }
    if focused {
        ui.painter().rect_stroke(
            rect.shrink(1.),
            4.,
            Stroke::new(1_f32, theme::BLUE),
            egui::StrokeKind::Inside,
        );
    }
    let color = if active { theme::TEXT } else { theme::MUTED };
    diagram_icon(
        ui.painter(),
        rect.min + egui::vec2(17., 17.),
        kind,
        if active { theme::BLUE } else { color },
    );
    text(
        ui,
        Rect::from_min_max(
            rect.min + egui::vec2(34., 0.),
            egui::pos2(close_rect.left() - 6., rect.bottom()),
        ),
        name,
        color,
    );
    if active || hover || focused {
        if close.hovered() || close.has_focus() {
            ui.painter().rect_filled(close_rect, 4., theme::HOVER);
        }
        let c = close_rect.center();
        for (a, b) in [([-3.5, -3.5], [3.5, 3.5]), ([-3.5, 3.5], [3.5, -3.5])] {
            ui.painter().line_segment(
                [c + egui::vec2(a[0], a[1]), c + egui::vec2(b[0], b[1])],
                Stroke::new(1.4_f32, theme::MUTED),
            );
        }
    }
    select.widget_info(|| {
        egui::WidgetInfo::selected(
            egui::WidgetType::SelectableLabel,
            ui.is_enabled(),
            active,
            name,
        )
    });
    close.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), "탭 닫기")
    });
    TabResponse { select, close }
}

pub fn list_row(
    ui: &mut egui::Ui,
    label: &str,
    selected: bool,
    kind: Option<DiagramKind>,
) -> Response {
    let (rect, response) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), theme::CONTROL_HEIGHT),
        Sense::click(),
    );
    ui.painter().rect_filled(
        rect,
        4.,
        if selected {
            theme::BLUE_LIGHT
        } else if response.hovered() || response.has_focus() {
            theme::HOVER
        } else {
            Color32::TRANSPARENT
        },
    );
    let color = if selected {
        theme::BLUE_TEXT
    } else {
        theme::TEXT
    };
    let inset = if let Some(kind) = kind {
        diagram_icon(
            ui.painter(),
            rect.min + egui::vec2(17., 16.),
            kind,
            if selected { theme::BLUE } else { theme::MUTED },
        );
        34.
    } else {
        10.
    };
    text(
        ui,
        Rect::from_min_max(
            rect.min + egui::vec2(inset, 0.),
            rect.max - egui::vec2(8., 0.),
        ),
        label,
        color,
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

pub fn section(ui: &mut egui::Ui, label: &str) {
    ui.add_space(4.);
    ui.separator();
    ui.label(egui::RichText::new(label).strong().size(13.));
}

pub fn delete_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add(
        egui::Button::new(egui::RichText::new(label).color(theme::ERROR))
            .fill(Color32::from_rgb(253, 242, 242)),
    )
}

pub fn text_input(value: &mut String) -> egui::TextEdit<'_> {
    egui::TextEdit::singleline(value)
        .desired_width(f32::INFINITY)
        .min_size(egui::vec2(0., theme::CONTROL_HEIGHT))
        .margin(egui::Margin::symmetric(8, 7))
}

pub fn field_label(ui: &mut egui::Ui, label: &str) {
    ui.label(egui::RichText::new(label).size(12.).color(theme::MUTED));
}

pub fn icon_button(ui: &mut egui::Ui, label: &str) -> Response {
    ui.add_sized(
        [theme::ICON_SIZE, theme::ICON_SIZE],
        egui::Button::new(label).min_size(egui::Vec2::ZERO),
    )
}

pub fn tool_hint(ui: &mut egui::Ui, label: &str) {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.), Sense::hover());
    let galley = egui::WidgetText::from(egui::RichText::new(label).size(12.)).into_galley(
        ui,
        Some(egui::TextWrapMode::Truncate),
        rect.width(),
        egui::TextStyle::Small,
    );
    ui.painter().galley(
        egui::pos2(rect.left(), rect.center().y - galley.size().y / 2.),
        galley,
        theme::MUTED,
    );
    response.on_hover_text(label);
}
