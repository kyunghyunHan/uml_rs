use crate::model::*;
use eframe::egui;
#[derive(Default)]
pub struct PropertiesState {
    pub focus_name: Option<Id>,
    pub open_diagram: Option<Id>,
    pub error: Option<(Id, String)>,
    pub draft: Option<(Id, Relation)>,
    quick: std::collections::HashMap<(Id, bool), QuickEntry>,
}
#[derive(Default)]
pub struct Catalog {
    pub types: Vec<Node>,
    pub sequences: Vec<(Id, String)>,
}
impl Catalog {
    pub fn from_project(p: &Project) -> Self {
        Self {
            types: p
                .diagrams
                .iter()
                .flat_map(|d| d.nodes.iter())
                .filter(|n| n.is_classifier())
                .cloned()
                .collect(),
            sequences: p
                .diagrams
                .iter()
                .filter(|d| d.kind == DiagramKind::Sequence)
                .map(|d| (d.id, d.name.clone()))
                .collect(),
        }
    }
}
fn field(ui: &mut egui::Ui, label: &str, s: &mut String) {
    crate::components::field_label(ui, label);
    ui.add(crate::components::text_input(s).desired_width(ui.available_width()));
}
fn enum_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: &mut T,
    options: &[T],
    name: impl Fn(T) -> String,
) {
    crate::components::field_label(ui, label);
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width())
        .truncate()
        .selected_text(name(*current))
        .show_ui(ui, |ui| {
            for &v in options {
                ui.selectable_value(current, v, name(v));
            }
        });
}
#[derive(Default)]
struct QuickEntry {
    text: String,
    error: Option<String>,
}
fn members_ui(ui: &mut egui::Ui, members: &mut Vec<Member>, method: bool, entry: &mut QuickEntry) {
    ui.horizontal_wrapped(|ui| {
        let input = ui.add(
            crate::components::text_input(&mut entry.text)
                .hint_text(if method {
                    "+ sort(values: List<int>): void"
                } else {
                    "- policy: SortPolicy"
                })
                .desired_width((ui.available_width() - 80.).clamp(60., 180.)),
        );
        let enter = input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
        if ui.button("추가").clicked() || enter {
            match Member::parse(&entry.text, method) {
                Ok(member) => {
                    members.push(member);
                    entry.text.clear();
                    entry.error = None;
                }
                Err(error) => entry.error = Some(error),
            }
        }
    });
    if let Some(error) = &entry.error {
        ui.colored_label(egui::Color32::DARK_RED, error);
    }

    let mut remove = None;
    let mut swap = None;
    for (i, m) in members.iter_mut().enumerate() {
        ui.push_id(m.id, |ui| {
            ui.group(|ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.add_sized(
                        [(ui.available_width() - 100.).max(70.), 24.],
                        egui::Label::new(egui::RichText::new(m.text(method)).strong()).truncate(),
                    )
                    .on_hover_text(m.text(method));
                    if crate::components::icon_button(ui, "↑").clicked() && i > 0 {
                        swap = Some((i, i - 1));
                    }
                    if crate::components::icon_button(ui, "↓").clicked() {
                        swap = Some((i, i + 1));
                    }
                    if crate::components::icon_button(ui, "×").clicked() {
                        remove = Some(i);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    egui::ComboBox::from_id_salt("visibility")
                        .width(45.)
                        .selected_text(&m.visibility)
                        .show_ui(ui, |ui| {
                            for v in ["+", "-", "#", "~"] {
                                ui.selectable_value(&mut m.visibility, v.into(), v);
                            }
                        });
                    ui.add(
                        crate::components::text_input(&mut m.name)
                            .desired_width(ui.available_width()),
                    );
                });
                field(
                    ui,
                    if method { "반환 타입" } else { "타입" },
                    &mut m.type_name,
                );
                if method {
                    field(ui, "매개변수", &mut m.parameters);
                } else {
                    field(ui, "기본값", &mut m.value);
                }
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut m.is_static, "static");
                    ui.checkbox(&mut m.is_abstract, "abstract");
                });
            });
        });
    }
    if let Some(i) = remove {
        members.remove(i);
    } else if let Some((a, b)) = swap
        && b < members.len()
    {
        members.swap(a, b);
    }
    if ui
        .button(if method { "+ 메서드" } else { "+ 속성" })
        .clicked()
    {
        members.push(Member::default());
    }
}
fn participant_combo(ui: &mut egui::Ui, label: &str, id: &mut Id, ps: &[Participant]) {
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width())
        .truncate()
        .selected_text(format!(
            "{label}: {}",
            ps.iter()
                .find(|p| p.id == *id)
                .map(|p| p.name.as_str())
                .unwrap_or("없음")
        ))
        .show_ui(ui, |ui| {
            for p in ps {
                ui.selectable_value(id, p.id, &p.name);
            }
        });
}
fn event_combo(ui: &mut egui::Ui, label: &str, id: &mut Id, ms: &[Message]) {
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width())
        .truncate()
        .selected_text(format!(
            "{label}: {}",
            ms.iter()
                .position(|m| m.id == *id)
                .map(|i| format!("{} {}", i + 1, ms[i].text))
                .unwrap_or_default()
        ))
        .show_ui(ui, |ui| {
            for (i, m) in ms.iter().enumerate() {
                ui.selectable_value(id, m.id, format!("{} {}", i + 1, m.text));
            }
        });
}
pub fn sequence_add(
    ui: &mut egui::Ui,
    s: &mut Sequence,
    selection: &mut std::collections::HashSet<Id>,
) {
    if crate::palette::sequence_button(ui, "참여자", 0, true).clicked() {
        let p = Participant {
            type_ref: None,
            id: id(),
            name: format!("참여자 {}", s.participants.len() + 1),
            actor: false,
        };
        selection.clear();
        selection.insert(p.id);
        s.participants.push(p);
    }
    if crate::palette::sequence_button(ui, "메시지", 1, !s.participants.is_empty()).clicked() {
        let m = Message {
            method_ref: None,
            id: id(),
            source: s.participants[0].id,
            target: s.participants.get(1).unwrap_or(&s.participants[0]).id,
            kind: MessageKind::Sync,
            text: "호출()".into(),
        };
        selection.clear();
        selection.insert(m.id);
        s.messages.push(m);
    }
    if crate::palette::sequence_button(ui, "활성 구간", 2, !s.messages.is_empty()).clicked() {
        let a = Activation {
            automatic: false,
            id: id(),
            participant: s.messages[0].target,
            start: s.messages[0].id,
            end: s.messages.last().unwrap().id,
        };
        selection.clear();
        selection.insert(a.id);
        s.activations.push(a);
    }
    if crate::palette::sequence_button(ui, "프래그먼트", 3, !s.messages.is_empty()).clicked() {
        let f = Fragment {
            participant_start: None,
            participant_end: None,
            id: id(),
            kind: FragmentKind::Alt,
            condition: "조건".into(),
            else_condition: "else".into(),
            start: s.messages[0].id,
            end: s.messages.last().unwrap().id,
            split: None,
        };
        selection.clear();
        selection.insert(f.id);
        s.fragments.push(f);
    }
}
#[derive(Clone, Copy)]
struct SequenceDrag {
    id: Id,
    participant: bool,
}
pub fn sequence_list(
    ui: &mut egui::Ui,
    s: &mut Sequence,
    selection: &mut std::collections::HashSet<Id>,
    search: &str,
) -> Option<Id> {
    let mut focus = None;
    let mut reorder = None;
    for (participant, rows) in [
        (
            true,
            s.participants
                .iter()
                .map(|p| (p.id, p.name.clone()))
                .collect::<Vec<_>>(),
        ),
        (
            false,
            s.messages
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id, format!("{} · {}", i + 1, m.text)))
                .collect(),
        ),
    ] {
        for (id, label) in rows {
            if !label.to_lowercase().contains(search) {
                continue;
            }
            let response = ui
                .dnd_drag_source(
                    egui::Id::new(("sequence-order", id)),
                    SequenceDrag { id, participant },
                    |ui| crate::components::list_row(ui, &label, selection.contains(&id), None),
                )
                .inner;
            if response.clicked() {
                focus = Some(id);
                if !ui.input(|i| i.modifiers.shift || i.modifiers.command) {
                    selection.clear();
                }
                selection.insert(id);
            }
            if response.double_clicked() {
                selection.clear();
                selection.insert(id);
            }
            if let Some(payload) = response.dnd_release_payload::<SequenceDrag>()
                && payload.participant == participant
            {
                reorder = Some((payload.id, id, participant));
            }
            response.on_hover_text("행을 드래그해 순서를 변경합니다 · Shift+클릭으로 다중 선택");
        }
    }
    if let Some((from, to, participant)) = reorder {
        if participant {
            if let (Some(a), Some(b)) = (
                s.participants.iter().position(|p| p.id == from),
                s.participants.iter().position(|p| p.id == to),
            ) {
                let p = s.participants.remove(a);
                s.participants.insert(b, p);
            }
        } else if let (Some(a), Some(b)) = (
            s.messages.iter().position(|m| m.id == from),
            s.messages.iter().position(|m| m.id == to),
        ) {
            let m = s.messages.remove(a);
            s.messages.insert(b, m);
            s.normalize();
        }
    }
    for (id, label) in s
        .activations
        .iter()
        .map(|a| (a.id, "활성 구간".to_string()))
        .chain(
            s.fragments
                .iter()
                .map(|f| (f.id, format!("{:?} [{}]", f.kind, f.condition))),
        )
    {
        if label.to_lowercase().contains(search)
            && crate::components::list_row(ui, &label, selection.contains(&id), None).clicked()
        {
            selection.clear();
            selection.insert(id);
            focus = Some(id);
        }
    }
    focus
}

pub fn properties(
    ui: &mut egui::Ui,
    d: &mut Diagram,
    selection: &mut std::collections::HashSet<Id>,
    catalog: &Catalog,
    state: &mut PropertiesState,
) {
    if selection.len() != 1 {
        ui.label(if selection.is_empty() {
            "요소 또는 연결선을 선택하세요".into()
        } else {
            format!("{}개 선택됨", selection.len())
        });
        return;
    }
    let selected = *selection.iter().next().unwrap();
    let parents: Vec<_> = d
        .nodes
        .iter()
        .filter(|n| matches!(n.kind, NodeKind::Boundary | NodeKind::Package))
        .map(|n| (n.id, n.name.clone()))
        .collect();
    let relation_draft = state
        .draft
        .as_ref()
        .filter(|(diagram, r)| *diagram == d.id && r.id == selected)
        .map(|(_, r)| r.clone())
        .or_else(|| d.relations.iter().find(|r| r.id == selected).cloned());
    let boundary_before = d
        .nodes
        .iter()
        .find(|n| n.id == selected && matches!(n.kind, NodeKind::Boundary | NodeKind::Package))
        .map(|n| n.layout);
    if let Some(n) = d.nodes.iter_mut().find(|n| n.id == selected) {
        ui.label(
            egui::RichText::new(n.kind.label())
                .strong()
                .color(crate::theme::BLUE_TEXT),
        );
        crate::components::field_label(
            ui,
            if n.kind == NodeKind::Note {
                "주석"
            } else {
                "이름"
            },
        );
        let edit = if n.kind == NodeKind::Note {
            ui.add(
                egui::TextEdit::multiline(&mut n.name)
                    .desired_width(ui.available_width())
                    .id_salt(n.id),
            )
        } else {
            ui.add(crate::components::text_input(&mut n.name).id_salt(n.id))
        };
        if state.focus_name == Some(n.id) {
            edit.request_focus();
            state.focus_name = None;
        }
        ui.collapsing("외형 · 배치", |ui| {
            let old_size = n.layout.size;
            ui.label("위치 / 크기");
            ui.horizontal_wrapped(|ui| {
                ui.add(egui::DragValue::new(&mut n.layout.pos.x).prefix("X "));
                ui.add(egui::DragValue::new(&mut n.layout.pos.y).prefix("Y "));
            });
            ui.horizontal_wrapped(|ui| {
                ui.add(
                    egui::DragValue::new(&mut n.layout.size.x)
                        .range(50.0..=20000.)
                        .prefix("W "),
                );
                ui.add(
                    egui::DragValue::new(&mut n.layout.size.y)
                        .range(50.0..=20000.)
                        .prefix("H "),
                );
            });
            if n.layout.size != old_size {
                n.sizing = SizeMode::Manual;
            }
        });
        if !matches!(n.kind, NodeKind::Boundary | NodeKind::Package) {
            crate::components::field_label(ui, "포함 경계 / 패키지");
            egui::ComboBox::from_id_salt("container")
                .width(ui.available_width())
                .truncate()
                .selected_text(
                    n.boundary
                        .and_then(|id| parents.iter().find(|p| p.0 == id).map(|p| p.1.as_str()))
                        .unwrap_or("경계/패키지 없음"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut n.boundary, None, "포함 관계 해제");
                    for (id, name) in &parents {
                        ui.selectable_value(&mut n.boundary, Some(*id), name);
                    }
                });
        }
        if n.is_classifier() {
            enum_combo(
                ui,
                "클래스 종류",
                &mut n.kind,
                &[
                    NodeKind::Class,
                    NodeKind::Interface,
                    NodeKind::AbstractClass,
                    NodeKind::Enumeration,
                ],
                |k| k.label().into(),
            );
            ui.horizontal_wrapped(|ui| {
                ui.radio_value(&mut n.sizing, SizeMode::Auto, "자동 크기");
                ui.radio_value(&mut n.sizing, SizeMode::Manual, "수동 크기");
            });
            ui.checkbox(&mut n.hide_empty, "빈 속성·메서드 구획 숨김");
            if ui.button("내용에 크기 맞춤").clicked() {
                n.fit();
            }

            ui.separator();
            ui.collapsing("속성", |ui| {
                members_ui(
                    ui,
                    &mut n.attributes,
                    false,
                    state.quick.entry((n.id, false)).or_default(),
                )
            });
            ui.collapsing("메서드", |ui| {
                members_ui(
                    ui,
                    &mut n.methods,
                    true,
                    state.quick.entry((n.id, true)).or_default(),
                )
            });
            n.reflow();
            if n.sizing == SizeMode::Manual
                && crate::text_layout::class_layout(n, Some(n.layout.size.x)).overflow
            {
                ui.colored_label(
                    egui::Color32::from_rgb(173, 76, 30),
                    "텍스트가 상자 높이를 넘습니다. 높이를 늘리거나 내용에 크기를 맞추세요.",
                );
            }
        }
        if n.kind == NodeKind::UseCase {
            specification_ui(ui, &mut n.specification, catalog, state);
        }
        n.reflow();
    } else if let Some(mut r) = relation_draft {
        ui.label(
            egui::RichText::new("관계 설정")
                .strong()
                .color(crate::theme::BLUE_TEXT),
        );
        crate::components::field_label(ui, "관계 종류");
        egui::ComboBox::from_id_salt("relation-kind")
            .width(ui.available_width())
            .selected_text(r.kind.label())
            .show_ui(ui, |ui| {
                for &kind in RelationKind::allowed(d.kind) {
                    if crate::palette::relation_option(
                        ui,
                        kind.label(),
                        kind,
                        NavigationPreset::Unspecified,
                        r.kind == kind,
                    )
                    .clicked()
                    {
                        r.kind = kind;
                        ui.close();
                    }
                }
            });
        let (snav, tnav) = (r.source_navigation, r.target_navigation);
        let preview = crate::scene::relation_preview(r.kind, snav, tnav);
        let (rect, _) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.), egui::Sense::hover());
        crate::scene::paint(
            &preview,
            ui.painter(),
            rect.min + egui::vec2(12., 12.),
            Point::default(),
            0.85,
        );
        field(ui, "관계 이름 / 라벨", &mut r.name);
        crate::components::section(ui, "연결 대상");
        if ui.button("시작↔도착 교환").clicked() {
            r = crate::commands::reversed_relation(d, &r);
        }
        let (source_label, target_label) = match r.kind {
            RelationKind::Generalization | RelationKind::Realization => {
                ("시작 · 하위 / 구현 타입", "도착 · 상위 타입")
            }
            RelationKind::Aggregation | RelationKind::Composition => {
                ("시작 · 전체 ◇", "도착 · 부분")
            }
            _ => ("시작", "도착"),
        };
        for (label, source) in [(source_label, true), (target_label, false)] {
            let current = if source { r.source } else { r.target };
            crate::components::field_label(ui, label);
            egui::ComboBox::from_id_salt(("endpoint", source))
                .width(ui.available_width())
                .truncate()
                .selected_text(
                    d.nodes
                        .iter()
                        .find(|n| n.id == current)
                        .map(|n| n.name.as_str())
                        .unwrap_or("요소 없음"),
                )
                .show_ui(ui, |ui| {
                    for node in &d.nodes {
                        let valid = crate::validation::endpoint_candidate(d, &r, node.id, source);
                        let response = ui.add_enabled(
                            valid.is_ok(),
                            egui::Button::new(&node.name)
                                .selected(node.id == current)
                                .truncate(),
                        );
                        let clicked = response.clicked();
                        response.on_hover_text(valid.err().unwrap_or_else(|| node.name.clone()));
                        if clicked {
                            if source {
                                r.source = node.id;
                            } else {
                                r.target = node.id;
                            }
                            ui.close();
                        }
                    }
                });
        }
        if d.kind == DiagramKind::Class && r.kind.has_navigation() {
            crate::components::section(ui, "탐색 방향");
            ui.menu_button("방향 프리셋", |ui| {
                for preset in NavigationPreset::ALL {
                    if crate::palette::relation_option(
                        ui,
                        preset.label(),
                        r.kind,
                        preset,
                        (r.source_navigation, r.target_navigation) == preset.ends(),
                    )
                    .clicked()
                    {
                        r.apply_navigation(preset);
                        ui.close();
                    }
                }
            });
            enum_combo(
                ui,
                "시작 끝 탐색 상태",
                &mut r.source_navigation,
                &Navigability::ALL,
                |n| n.label().into(),
            );
            enum_combo(
                ui,
                "도착 끝 탐색 상태",
                &mut r.target_navigation,
                &Navigability::ALL,
                |n| n.label().into(),
            );
            ui.small("미지정은 표시하지 않습니다. 탐색 불가는 x로 명시합니다. 다이아몬드는 전체(시작) 쪽에 유지됩니다.");
        }
        ui.collapsing("역할 · 다중성", |ui| {
            field(ui, "시작 역할", &mut r.source_role);
            multiplicity_field(ui, "시작 다중성", &mut r.source_multiplicity);
            field(ui, "도착 역할", &mut r.target_role);
            multiplicity_field(ui, "도착 다중성", &mut r.target_multiplicity);
        });
        crate::components::section(ui, "경로");
        enum_combo(
            ui,
            "경로 모드",
            &mut r.route_mode,
            &RouteMode::ALL,
            |mode| mode.label().into(),
        );
        if !crate::routing::path_obstructions(d, &r).is_empty() {
            ui.colored_label(
                crate::theme::ERROR,
                "경로가 요소 내부를 지나갑니다. 연결점·경로 모드 또는 배치를 조정하세요.",
            );
        }
        if r.bend.is_some() || !r.waypoints.is_empty() {
            ui.small("수동 경로를 우선 유지합니다. 모드에 따라 다시 계산하려면 자동 경로 초기화를 선택하세요.");
        }
        if d.kind == DiagramKind::Class {
            for (label, port) in [
                ("시작 연결점", &mut r.source_port),
                ("도착 연결점", &mut r.target_port),
            ] {
                let old = port.side;
                enum_combo(ui, label, &mut port.side, &PortSide::ALL, |side| {
                    side.label().into()
                });
                if old != port.side {
                    port.offset = 0.5;
                }
                if port.side != PortSide::Auto {
                    ui.add(
                        egui::DragValue::new(&mut port.offset)
                            .range(0.0..=1.)
                            .speed(0.01)
                            .prefix("테두리 위치 "),
                    );
                }
            }
            ui.small("선택한 관계의 양 끝 원을 드래그하면 테두리에 붙어 이동하고 가까운 연결점에 정렬됩니다.");
        }
        if r.waypoints.is_empty() {
            let mut manual = r.bend.is_some();
            if ui.checkbox(&mut manual, "직교 경로 수동 조절").changed() {
                r.bend = if manual { Some(300.) } else { None };
            }
            if let Some(bend) = &mut r.bend {
                ui.add(egui::DragValue::new(bend).prefix("중간 축 좌표 "));
            }
        }
        ui.collapsing("경로 조절점", |ui| {
            let mut remove = None;
            for (i, point) in r.waypoints.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}", i + 1));
                        ui.add(egui::DragValue::new(&mut point.x).prefix("X "));
                        ui.add(egui::DragValue::new(&mut point.y).prefix("Y "));
                        if crate::components::delete_button(ui, "삭제").clicked() {
                            remove = Some(i);
                        }
                    });
                });
            }
            if let Some(i) = remove {
                r.waypoints.remove(i);
            }
            if ui.button("조절점 추가").clicked() {
                let point = r
                    .waypoints
                    .last()
                    .map(|p| Point::new(p.x + 40., p.y + 40.))
                    .or_else(|| {
                        let pts = crate::routing::path(d, &r);
                        pts.windows(2)
                            .next()
                            .map(|w| Point::new((w[0].x + w[1].x) / 2., (w[0].y + w[1].y) / 2.))
                    })
                    .unwrap_or(Point::new(300., 200.));
                r.waypoints.push(point);
                r.bend = None;
            }
        });
        ui.small("캔버스에서 파란 조절점을 드래그해 경로를 편집합니다.");
        if ui.button("자동 경로 초기화").clicked() {
            r.reset_route();
        }
        let changed = d
            .relations
            .iter()
            .find(|a| a.id == r.id)
            .is_some_and(|old| *old != r);
        if changed {
            match d.update_relation(r.clone()) {
                Ok(()) => {
                    state.error = None;
                    state.draft = None;
                }
                Err(e) => {
                    state.error = Some((selected, e));
                    state.draft = Some((d.id, r));
                }
            }
        }
        if state
            .draft
            .as_ref()
            .is_some_and(|(id, r)| *id == d.id && r.id == selected)
            && ui.button("관계 수정 취소").clicked()
        {
            state.draft = None;
            state.error = None;
        }

        if let Some((id, error)) = &state.error
            && *id == selected
        {
            ui.colored_label(egui::Color32::DARK_RED, error);
        }
        if let Some(old) = d.relations.iter().find(|r| r.id == selected)
            && let Err(error) = crate::validation::relation(d, old)
        {
            ui.colored_label(egui::Color32::DARK_RED, error);
        }
    } else if d.kind == DiagramKind::Sequence {
        let s = &mut d.sequence;
        if let Some(i) = s.participants.iter().position(|p| p.id == selected) {
            let p = &mut s.participants[i];
            ui.label("참여자 이름");
            let edit = ui.add_enabled(
                p.type_ref.is_none(),
                crate::components::text_input(&mut p.name).id_salt((p.id, "rename")),
            );
            if state.focus_name == Some(p.id) {
                edit.request_focus();
                state.focus_name = None;
            }
            if p.type_ref.is_some() {
                ui.small("연결 타입의 이름을 사용합니다. 직접 수정하려면 연결을 해제하세요.");
            }
            type_combo(ui, &mut p.type_ref, catalog);
            ui.checkbox(&mut p.actor, "액터 형태");
            let mut swap = None;
            ui.horizontal_wrapped(|ui| {
                if ui.button("← 왼쪽").clicked() && i > 0 {
                    swap = Some(i - 1);
                }
                if ui.button("오른쪽 →").clicked() {
                    swap = Some(i + 1);
                }
            });
            if let Some(j) = swap
                && j < s.participants.len()
            {
                s.participants.swap(i, j);
            }
        } else if let Some(i) = s.messages.iter().position(|m| m.id == selected) {
            let m = &mut s.messages[i];
            ui.label("메시지");
            let edit = ui.add_enabled(
                m.method_ref.is_none(),
                crate::components::text_input(&mut m.text).id_salt((m.id, "rename")),
            );
            if state.focus_name == Some(m.id) {
                edit.request_focus();
                state.focus_name = None;
            }
            if m.method_ref.is_some() {
                ui.small("연결 메서드의 이름과 시그니처를 사용합니다.");
            }
            let target_type = s
                .participants
                .iter()
                .find(|p| p.id == m.target)
                .and_then(|p| p.type_ref);
            method_combo(ui, &mut m.method_ref, target_type, catalog);
            participant_combo(ui, "발신", &mut m.source, &s.participants);
            participant_combo(ui, "수신", &mut m.target, &s.participants);
            enum_combo(
                ui,
                "메시지 종류",
                &mut m.kind,
                &MessageKind::ALL,
                |k| k.label().into(),
            );
            let mut swap = None;
            let mut insert = None;
            ui.horizontal_wrapped(|ui| {
                if ui.button("↑").clicked() && i > 0 {
                    swap = Some(i - 1);
                }
                if ui.button("↓").clicked() {
                    swap = Some(i + 1);
                }
            });
            ui.horizontal_wrapped(|ui| {
                if ui.button("위에 삽입").clicked() {
                    insert = Some(i);
                }
                if ui.button("아래에 삽입").clicked() {
                    insert = Some(i + 1);
                }
            });
            if let Some(j) = swap
                && j < s.messages.len()
            {
                s.messages.swap(i, j);
            }
            if let Some(j) = insert {
                let mut m = s.messages[i].clone();
                m.id = id();
                m.text = "새 메시지".into();
                selection.clear();
                selection.insert(m.id);
                s.messages.insert(j, m);
            }
            s.normalize();
        } else if let Some(a) = s.activations.iter_mut().find(|a| a.id == selected) {
            if a.automatic {
                ui.label("자동으로 생성된 활성 구간");
                if ui.button("수동 편집으로 전환").clicked() {
                    a.automatic = false;
                }
            }
            participant_combo(ui, "참여자", &mut a.participant, &s.participants);
            event_combo(ui, "시작", &mut a.start, &s.messages);
            event_combo(ui, "종료", &mut a.end, &s.messages);
            s.normalize();
        } else if let Some(f) = s.fragments.iter_mut().find(|f| f.id == selected) {
            enum_combo(
                ui,
                "프래그먼트 종류",
                &mut f.kind,
                &[FragmentKind::Alt, FragmentKind::Opt, FragmentKind::Loop],
                |k| format!("{k:?}"),
            );
            field(ui, "조건", &mut f.condition);
            range_combo(ui, "왼쪽 참여자", &mut f.participant_start, &s.participants);
            range_combo(ui, "오른쪽 참여자", &mut f.participant_end, &s.participants);
            event_combo(ui, "시작", &mut f.start, &s.messages);
            event_combo(ui, "종료", &mut f.end, &s.messages);
            if f.kind == FragmentKind::Alt {
                field(ui, "else 조건", &mut f.else_condition);
                let mut explicit = f.split.is_some();
                if ui
                    .checkbox(&mut explicit, "분기 위치를 이벤트에 연결")
                    .changed()
                {
                    f.split = if explicit { Some(f.start) } else { None };
                }
                if let Some(split) = &mut f.split {
                    event_combo(ui, "분기", split, &s.messages);
                }
            }
            s.normalize();
        }
    }
    if let Some(old) = boundary_before {
        let new = d.nodes.iter().find(|n| n.id == selected).unwrap().layout;
        let delta = Point::new(new.pos.x - old.pos.x, new.pos.y - old.pos.y);
        for child in &mut d.nodes {
            if child.boundary == Some(selected) {
                child.layout.pos.x += delta.x;
                child.layout.pos.y += delta.y;
            }
        }
    }
    ui.add_space(16.);
    ui.separator();
    if crate::components::delete_button(ui, "선택 삭제").clicked() {
        d.delete(selection);
        selection.clear();
    }
}

fn type_combo(ui: &mut egui::Ui, current: &mut Option<Id>, catalog: &Catalog) {
    egui::ComboBox::from_id_salt("type-ref")
        .width(ui.available_width())
        .truncate()
        .selected_text(
            current
                .and_then(|id| {
                    catalog
                        .types
                        .iter()
                        .find(|n| n.id == id)
                        .map(|n| n.name.as_str())
                })
                .unwrap_or(if current.is_some() {
                    "타입 참조 없음 (해제 필요)"
                } else {
                    "자유 참여자"
                }),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(current, None, "자유 참여자 / 연결 해제");
            for n in &catalog.types {
                ui.selectable_value(current, Some(n.id), &n.name);
            }
        });
}
fn method_combo(
    ui: &mut egui::Ui,
    current: &mut Option<Id>,
    type_ref: Option<Id>,
    catalog: &Catalog,
) {
    let methods = catalog
        .types
        .iter()
        .find(|n| Some(n.id) == type_ref)
        .map(|n| n.methods.as_slice())
        .unwrap_or(&[]);
    egui::ComboBox::from_id_salt("method-ref")
        .width(ui.available_width())
        .truncate()
        .selected_text(
            current
                .and_then(|id| methods.iter().find(|m| m.id == id).map(|m| m.name.as_str()))
                .unwrap_or(if current.is_some() {
                    "메서드 참조 없음 (해제 필요)"
                } else {
                    "자유 메시지"
                }),
        )
        .show_ui(ui, |ui| {
            ui.selectable_value(current, None, "자유 메시지 / 연결 해제");
            for m in methods {
                ui.selectable_value(current, Some(m.id), m.text(true));
            }
        });
}
fn range_combo(ui: &mut egui::Ui, label: &str, current: &mut Option<Id>, ps: &[Participant]) {
    egui::ComboBox::from_id_salt(label)
        .width(ui.available_width())
        .truncate()
        .selected_text(format!(
            "{label}: {}",
            current
                .and_then(|id| ps.iter().find(|p| p.id == id).map(|p| p.name.as_str()))
                .unwrap_or("전체")
        ))
        .show_ui(ui, |ui| {
            ui.selectable_value(current, None, "전체 범위");
            for p in ps {
                ui.selectable_value(current, Some(p.id), &p.name);
            }
        });
}
fn specification_ui(
    ui: &mut egui::Ui,
    spec: &mut UseCaseSpec,
    catalog: &Catalog,
    state: &mut PropertiesState,
) {
    egui::CollapsingHeader::new("유스케이스 명세")
        .default_open(true)
        .show(ui, |ui| {
            for (label, value) in [
                ("목표", &mut spec.goal),
                ("설명", &mut spec.description),
                ("주요 액터", &mut spec.primary_actor),
                ("사전조건", &mut spec.preconditions),
                ("기본 흐름", &mut spec.basic_flow),
                ("대안·예외 흐름", &mut spec.alternatives),
                ("사후조건", &mut spec.postconditions),
            ] {
                ui.label(label);
                ui.add(
                    egui::TextEdit::multiline(value)
                        .desired_width(ui.available_width())
                        .desired_rows(2),
                );
            }
            ui.strong("관련 시퀀스");
            for (id, name) in &catalog.sequences {
                ui.horizontal_wrapped(|ui| {
                    let mut linked = spec.sequences.contains(id);
                    let label_width = (ui.available_width() - 55.).max(40.);
                    if ui
                        .scope(|ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                            ui.add_sized(
                                [label_width, ui.spacing().interact_size.y],
                                egui::Checkbox::new(&mut linked, name),
                            )
                        })
                        .inner
                        .on_hover_text(name)
                        .changed()
                    {
                        if linked {
                            spec.sequences.push(*id);
                        } else {
                            spec.sequences.retain(|old| old != id);
                        }
                    }
                    if linked && ui.button("열기").clicked() {
                        state.open_diagram = Some(*id);
                    }
                });
            }
            let missing: Vec<_> = spec
                .sequences
                .iter()
                .filter(|id| !catalog.sequences.iter().any(|n| n.0 == **id))
                .copied()
                .collect();
            for id in missing {
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(egui::Color32::DARK_RED, "삭제된 시퀀스 참조");
                    if ui.button("연결 해제").clicked() {
                        spec.sequences.retain(|old| *old != id);
                    }
                });
            }
        });
}

fn multiplicity_field(ui: &mut egui::Ui, label: &str, value: &mut String) {
    field(ui, label, value);
    ui.push_id(label, |ui| {
        ui.menu_button("다중성 프리셋", |ui| {
            for preset in ["", "0", "1", "0..1", "*", "1..*", "2..5"] {
                if ui
                    .button(if preset.is_empty() {
                        "미지정"
                    } else {
                        preset
                    })
                    .clicked()
                {
                    *value = preset.into();
                    ui.close();
                }
            }
        });
    });
    if let Err(error) = crate::validation::multiplicity(value) {
        ui.colored_label(crate::theme::ERROR, error);
    }
}
