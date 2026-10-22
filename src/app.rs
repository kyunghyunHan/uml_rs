use crate::{
    canvas::{self, CanvasState, Tool},
    commands::{self, Clipboard, History},
    model::*,
    scene::{self, ExportOptions},
    storage::{self, Preferences},
};
use eframe::egui::{self, Color32, Key, RichText};
use std::{
    path::PathBuf,
    time::{Duration, Instant},
};
#[derive(Clone)]
enum Action {
    New,
    Open(PathBuf),
    Home,
    Quit,
    Example,
    Recover,
}
pub struct UmlApp {
    pub project: Option<Project>,
    saved: Option<Project>,
    path: Option<PathBuf>,
    active: usize,
    tabs: Vec<Id>,
    history: History,
    clipboard: Clipboard,
    canvas: CanvasState,
    tool: Tool,
    prefs: Preferences,
    status: String,
    error: Option<String>,
    pending: Option<Action>,
    allow_close: bool,
    new_dialog: bool,
    new_name: String,
    new_diagram: String,
    new_kind: DiagramKind,
    add_dialog: bool,
    export_dialog: bool,
    export_options: ExportOptions,
    transaction: Option<Project>,
    last_recovery: Instant,
    recovery_available: bool,
    canvas_size: egui::Vec2,
    suppress_history: bool,
    blocked: bool,
    canvas_hidden: bool,
}
impl UmlApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "korean".into(),
            egui::FontData::from_static(scene::FONT).into(),
        );
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "korean".into());
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "korean".into());
        cc.egui_ctx.set_fonts(fonts);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        let mut style = (*cc.egui_ctx.style()).clone();
        style.spacing.item_spacing = egui::vec2(8., 8.);
        style.spacing.button_padding = egui::vec2(10., 6.);
        style.visuals.selection.bg_fill = Color32::from_rgb(66, 105, 218);
        cc.egui_ctx.set_style(style);
        Self {
            project: None,
            saved: None,
            path: None,
            active: 0,
            tabs: vec![],
            history: History::default(),
            clipboard: Clipboard::default(),
            canvas: CanvasState::new(),
            tool: Tool::Select,
            prefs: Preferences::load(),
            status: "준비됨".into(),
            error: None,
            pending: None,
            allow_close: false,
            new_dialog: false,
            new_name: "새 프로젝트".into(),
            new_diagram: "다이어그램 1".into(),
            new_kind: DiagramKind::Class,
            add_dialog: false,
            export_dialog: false,
            export_options: ExportOptions::default(),
            transaction: None,
            last_recovery: Instant::now(),
            recovery_available: storage::config_dir().join("recovery.umlproj").exists(),
            canvas_size: egui::vec2(800., 600.),
            suppress_history: false,
            blocked: false,
            canvas_hidden: false,
        }
    }
    fn dirty(&self) -> bool {
        self.project.is_some() && self.project != self.saved
    }
    fn commit(&mut self) {
        if let (Some(before), Some(p)) = (self.transaction.take(), self.project.as_ref()) {
            self.history.record(before, p);
        }
    }
    fn replace(&mut self, p: Project, path: Option<PathBuf>, clean: bool) {
        self.suppress_history = true;
        self.active = 0;
        self.canvas_hidden = false;
        self.tabs = p.diagrams.first().map(|d| vec![d.id]).unwrap_or_default();
        self.saved = if clean { Some(p.clone()) } else { None };
        self.project = Some(p);
        self.path = path;
        self.history = History::default();
        self.transaction = None;
        self.canvas.clear();
        self.tool = Tool::Select;
    }
    fn save(&mut self, save_as: bool) -> bool {
        self.commit();
        if self.project.is_none() {
            return false;
        }
        let path = if save_as || self.path.is_none() {
            rfd::FileDialog::new()
                .add_filter("UML 프로젝트", &["umlproj"])
                .set_file_name(format!("{}.umlproj", self.project.as_ref().unwrap().name))
                .save_file()
                .map(|mut p| {
                    p.set_extension("umlproj");
                    p
                })
        } else {
            self.path.clone()
        };
        if let Some(path) = path {
            match storage::save(&path, self.project.as_ref().unwrap()) {
                Ok(()) => {
                    self.saved = self.project.clone();
                    self.path = Some(path.clone());
                    self.prefs.remember(&path);
                    self.status = format!("저장됨 · {}", path.display());
                    let _ = std::fs::remove_file(storage::config_dir().join("recovery.umlproj"));
                    self.recovery_available = false;
                    true
                }
                Err(e) => {
                    self.error = Some(e);
                    false
                }
            }
        } else {
            false
        }
    }
    fn request(&mut self, a: Action, ctx: &egui::Context) {
        if self.dirty() {
            self.pending = Some(a);
        } else {
            self.perform(a, ctx);
        }
    }
    fn perform(&mut self, a: Action, ctx: &egui::Context) {
        match a {
            Action::New => self.new_dialog = true,
            Action::Home => {
                self.project = None;
                self.transaction = None;
            }
            Action::Quit => {
                self.allow_close = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Action::Example => self.replace(example_project(), None, false),
            Action::Open(path) => match storage::load(&path) {
                Ok(p) => {
                    self.replace(p, Some(path.clone()), true);
                    self.prefs.remember(&path);
                    self.status = "프로젝트를 열었습니다".into();
                }
                Err(e) => self.error = Some(e),
            },
            Action::Recover => match storage::load(&storage::config_dir().join("recovery.umlproj"))
            {
                Ok(p) => {
                    self.replace(p, None, false);
                    self.recovery_available = false;
                    self.status = "복구됨 · 다른 이름으로 저장하세요".into();
                }
                Err(e) => self.error = Some(e),
            },
        }
    }
    fn open(&mut self, ctx: &egui::Context) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("UML 프로젝트", &["umlproj"])
            .pick_file()
        {
            self.request(Action::Open(path), ctx);
        }
    }
    fn undo(&mut self) {
        self.suppress_history = true;
        self.commit();
        if let Some(p) = &mut self.project {
            self.history.undo(p);
            self.active = self.active.min(p.diagrams.len().saturating_sub(1));
            self.canvas.clear();
        }
    }
    fn redo(&mut self) {
        self.suppress_history = true;
        self.commit();
        if let Some(p) = &mut self.project {
            self.history.redo(p);
            self.active = self.active.min(p.diagrams.len().saturating_sub(1));
            self.canvas.clear();
        }
    }
    fn shortcuts(&mut self, ctx: &egui::Context) {
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::S)) {
                self.save(ctx.input(|i| i.modifiers.shift));
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::O)) {
                self.open(ctx);
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::N)) {
                self.request(Action::New, ctx);
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Z)) {
                if ctx.input(|i| i.modifiers.shift) {
                    self.redo();
                } else {
                    self.undo();
                }
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::Y)) {
                self.redo();
            }
            if !self.canvas_hidden
                && let Some(d) = self
                    .project
                    .as_mut()
                    .and_then(|p| p.diagrams.get_mut(self.active))
            {
                if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
                    d.delete(&self.canvas.selection);
                    self.canvas.selection.clear();
                }
                if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::A)) {
                    self.canvas.selection = d
                        .nodes
                        .iter()
                        .map(|n| n.id)
                        .chain(d.sequence.participants.iter().map(|p| p.id))
                        .chain(d.sequence.messages.iter().map(|m| m.id))
                        .chain(d.sequence.activations.iter().map(|a| a.id))
                        .chain(d.sequence.fragments.iter().map(|f| f.id))
                        .collect();
                }
                if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::C)) {
                    self.clipboard = Clipboard::copy(d, &self.canvas.selection);
                }
                if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::V)) {
                    self.canvas.selection = self.clipboard.paste(d);
                }
                if ctx.input(|i| i.modifiers.command && i.key_pressed(Key::D)) {
                    self.clipboard = Clipboard::copy(d, &self.canvas.selection);
                    self.canvas.selection = self.clipboard.paste(d);
                }
                if ctx.input(|i| i.key_pressed(Key::F)) {
                    canvas::fit(d, self.canvas_size);
                }
            }
        }
    }
    fn toolbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("toolbar").show(ctx, |ui| {
            if self.blocked {
                ui.disable();
            }
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new("UML STUDIO")
                        .strong()
                        .color(Color32::from_rgb(47, 79, 172)),
                );
                ui.separator();
                ui.menu_button("파일", |ui| {
                    if ui.button("새 프로젝트    ⌘N").clicked() {
                        self.request(Action::New, ctx);
                        ui.close();
                    }
                    if ui.button("프로젝트 열기    ⌘O").clicked() {
                        self.open(ctx);
                        ui.close();
                    }
                    if ui.button("저장    ⌘S").clicked() {
                        self.save(false);
                        ui.close();
                    }
                    if ui.button("다른 이름으로 저장    ⇧⌘S").clicked() {
                        self.save(true);
                        ui.close();
                    }
                    if ui.button("시작 화면").clicked() {
                        self.request(Action::Home, ctx);
                        ui.close();
                    }
                });
                let has = self.project.is_some();
                if ui.add_enabled(has, egui::Button::new("저장")).clicked() {
                    self.save(false);
                }
                if ui
                    .add_enabled(
                        has && (self.history.can_undo() || self.transaction.is_some()),
                        egui::Button::new("실행 취소"),
                    )
                    .clicked()
                {
                    self.undo();
                }
                if ui
                    .add_enabled(
                        has && self.history.can_redo(),
                        egui::Button::new("다시 실행"),
                    )
                    .clicked()
                {
                    self.redo();
                }
                if ui
                    .add_enabled(has, egui::Button::new("PNG 내보내기"))
                    .clicked()
                {
                    self.export_dialog = true;
                }
                if let Some(p) = &self.project {
                    ui.separator();
                    ui.label(format!(
                        "{}{}",
                        p.name,
                        if self.dirty() { " •" } else { "" }
                    ));
                }
            });
        });
        egui::TopBottomPanel::bottom("status").show(ctx, |ui| {
            if self.blocked {
                ui.disable();
            }
            ui.horizontal(|ui| {
                ui.small(&self.status);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.small("F 맞춤 · ⌘/Ctrl+휠 확대 · 우클릭 드래그 이동 · Shift 다중 선택");
                });
            });
        });
    }
    fn home(&mut self, ctx: &egui::Context) {
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.blocked {
                ui.disable();
            }
            ui.vertical_centered(|ui| {
                ui.add_space(70.);
                ui.heading(RichText::new("UML Studio").size(40.));
                ui.label("아이디어를 구조와 흐름으로 정리하세요");
                ui.add_space(25.);
                if ui.button("새 프로젝트 만들기").clicked() {
                    self.new_dialog = true;
                }
                if ui.button("프로젝트 열기…").clicked() {
                    self.open(ctx);
                }
                if ui.button("‘정렬 프로그램’ 예제 열기").clicked() {
                    self.perform(Action::Example, ctx);
                }
                ui.add_space(25.);
                if self.recovery_available {
                    ui.group(|ui| {
                        ui.label("자동 복구 파일이 있습니다");
                        if ui.button("복구하기").clicked() {
                            self.perform(Action::Recover, ctx);
                        }
                        if ui.button("복구 파일 버리기").clicked() {
                            let _ = std::fs::remove_file(
                                storage::config_dir().join("recovery.umlproj"),
                            );
                            self.recovery_available = false;
                        }
                    });
                }
                ui.heading("최근 프로젝트");
                for path in self.prefs.recent.clone() {
                    if ui.button(path.display().to_string()).clicked() {
                        self.perform(Action::Open(path), ctx);
                    }
                }
            });
        });
    }
    fn editor(&mut self, ctx: &egui::Context) {
        let mut select = None;
        let mut remove = None;
        egui::SidePanel::left("diagrams")
            .default_width(220.)
            .min_width(170.)
            .show(ctx, |ui| {
                if self.blocked {
                    ui.disable();
                }
                ui.heading("프로젝트");
                if let Some(p) = &mut self.project {
                    ui.text_edit_singleline(&mut p.name);
                    ui.separator();
                    ui.horizontal(|ui| {
                        ui.strong("다이어그램");
                        if ui.small_button("+ 추가").clicked() {
                            self.add_dialog = true;
                        }
                    });
                    for (i, d) in p.diagrams.iter().enumerate() {
                        ui.horizontal(|ui| {
                            if ui
                                .selectable_label(
                                    i == self.active,
                                    format!(
                                        "{}  {}",
                                        match d.kind {
                                            DiagramKind::Class => "▤",
                                            DiagramKind::UseCase => "◉",
                                            DiagramKind::Sequence => "⇢",
                                        },
                                        d.name
                                    ),
                                )
                                .clicked()
                            {
                                select = Some(i);
                            }
                            if ui
                                .small_button("×")
                                .on_hover_text("다이어그램 삭제 (실행 취소 가능)")
                                .clicked()
                            {
                                remove = Some(i);
                            }
                        });
                    }
                    if let Some(d) = p.diagrams.get_mut(self.active) {
                        ui.separator();
                        ui.label("다이어그램 이름");
                        ui.text_edit_singleline(&mut d.name);
                        ui.add_space(15.);
                        ui.label("요소 목록");
                        egui::ScrollArea::vertical()
                            .id_salt("element-list")
                            .show(ui, |ui| {
                                for n in &d.nodes {
                                    if ui
                                        .selectable_label(
                                            self.canvas.selection.contains(&n.id),
                                            &n.name,
                                        )
                                        .clicked()
                                    {
                                        self.canvas.selection.clear();
                                        self.canvas.selection.insert(n.id);
                                    }
                                }
                                for r in &d.relations {
                                    if ui
                                        .selectable_label(
                                            self.canvas.selection.contains(&r.id),
                                            format!("↳ {} {}", r.kind.label(), r.name),
                                        )
                                        .clicked()
                                    {
                                        self.canvas.selection.clear();
                                        self.canvas.selection.insert(r.id);
                                    }
                                }
                                if d.kind == DiagramKind::Sequence {
                                    sequence_list(ui, &mut d.sequence, &mut self.canvas.selection);
                                }
                            });
                    }
                }
            });
        if let Some(i) = remove {
            self.commit();
            let p = self.project.as_mut().unwrap();
            let id = p.diagrams.remove(i).id;
            self.tabs.retain(|t| *t != id);
            self.active = self.active.min(p.diagrams.len().saturating_sub(1));
            self.canvas.clear();
        }
        if let Some(i) = select {
            self.commit();
            self.active = i;
            self.canvas_hidden = false;
            self.canvas.clear();
            self.tool = Tool::Select;
            if let Some(d) = self.project.as_ref().and_then(|p| p.diagrams.get(i))
                && !self.tabs.contains(&d.id)
            {
                self.tabs.push(d.id);
            }
        }
        egui::SidePanel::right("properties")
            .default_width(310.)
            .min_width(260.)
            .show(ctx, |ui| {
                if self.blocked {
                    ui.disable();
                }
                ui.heading("속성");
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if let Some(d) = self
                        .project
                        .as_mut()
                        .and_then(|p| p.diagrams.get_mut(self.active))
                    {
                        properties(ui, d, &mut self.canvas.selection);
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(247, 249, 252)))
            .show(ctx, |ui| {
                if self.blocked {
                    ui.disable();
                }
                let mut tab_select = None;
                let mut tab_close = None;
                if let Some(p) = &self.project {
                    ui.horizontal_wrapped(|ui| {
                        for id in &self.tabs {
                            if let Some((i, d)) =
                                p.diagrams.iter().enumerate().find(|(_, d)| d.id == *id)
                            {
                                if ui.selectable_label(i == self.active, &d.name).clicked() {
                                    tab_select = Some(i);
                                }
                                if ui.small_button("×").clicked() {
                                    tab_close = Some(*id);
                                }
                            }
                        }
                    });
                }
                if let Some(id) = tab_close {
                    self.tabs.retain(|x| *x != id);
                    if self.tabs.is_empty() {
                        self.canvas_hidden = true;
                        self.canvas.clear();
                    }
                    if let Some(d) = self
                        .project
                        .as_ref()
                        .and_then(|p| p.diagrams.get(self.active))
                        && d.id == id
                        && let Some(next) = self.tabs.last()
                    {
                        tab_select = self
                            .project
                            .as_ref()
                            .unwrap()
                            .diagrams
                            .iter()
                            .position(|d| d.id == *next);
                    }
                }
                if let Some(i) = tab_select {
                    self.active = i;
                    self.canvas_hidden = false;
                    self.canvas.clear();
                    self.tool = Tool::Select;
                }
                if self.canvas_hidden {
                    ui.centered_and_justified(|ui| {
                        ui.label("왼쪽 목록에서 다이어그램을 열어주세요");
                    });
                    return;
                }
                let Some(d) = self
                    .project
                    .as_mut()
                    .and_then(|p| p.diagrams.get_mut(self.active))
                else {
                    ui.centered_and_justified(|ui| {
                        if ui.button("다이어그램 추가").clicked() {
                            self.add_dialog = true;
                        }
                    });
                    return;
                };
                if !self.tabs.contains(&d.id) {
                    self.tabs.push(d.id);
                }
                ui.horizontal_wrapped(|ui| {
                    ui.selectable_value(&mut self.tool, Tool::Select, "선택");
                    let kinds: Vec<_> = match d.kind {
                        DiagramKind::Class => vec![
                            NodeKind::Class,
                            NodeKind::Interface,
                            NodeKind::AbstractClass,
                        ],
                        DiagramKind::UseCase => {
                            vec![NodeKind::Actor, NodeKind::UseCase, NodeKind::Boundary]
                        }
                        DiagramKind::Sequence => vec![],
                    };
                    for k in kinds {
                        ui.selectable_value(
                            &mut self.tool,
                            Tool::Add(k),
                            format!("+ {}", k.label()),
                        );
                    }
                    if d.kind != DiagramKind::Sequence {
                        ui.menu_button("관계 연결", |ui| {
                            for k in RelationKind::ALL {
                                if (d.kind == DiagramKind::Class
                                    && matches!(k, RelationKind::Include | RelationKind::Extend))
                                    || (d.kind == DiagramKind::UseCase
                                        && !matches!(
                                            k,
                                            RelationKind::Association
                                                | RelationKind::Include
                                                | RelationKind::Extend
                                                | RelationKind::Generalization
                                        ))
                                {
                                    continue;
                                }
                                if ui
                                    .selectable_label(self.tool == Tool::Connect(k), k.label())
                                    .clicked()
                                {
                                    self.tool = Tool::Connect(k);
                                    self.canvas.connection = None;
                                    ui.close();
                                }
                            }
                        });
                    } else {
                        sequence_add(ui, &mut d.sequence, &mut self.canvas.selection);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.canvas.grid, "그리드");
                    ui.checkbox(&mut self.canvas.snap, "맞춤");
                    if ui.button("화면 맞춤").clicked() {
                        canvas::fit(d, self.canvas_size);
                    }
                    ui.label(format!("{:.0}%", d.view.zoom * 100.));
                    ui.menu_button("선택 편집", |ui| {
                        if ui.button("삭제").clicked() {
                            d.delete(&self.canvas.selection);
                            self.canvas.selection.clear();
                            ui.close();
                        }
                        if ui.button("복사").clicked() {
                            self.clipboard = Clipboard::copy(d, &self.canvas.selection);
                            ui.close();
                        }
                        if ui.button("붙여넣기").clicked() {
                            self.canvas.selection = self.clipboard.paste(d);
                            ui.close();
                        }
                        if ui.button("복제").clicked() {
                            self.clipboard = Clipboard::copy(d, &self.canvas.selection);
                            self.canvas.selection = self.clipboard.paste(d);
                            ui.close();
                        }
                        for (i, label) in [
                            "왼쪽 정렬",
                            "오른쪽 정렬",
                            "위쪽 정렬",
                            "아래쪽 정렬",
                            "가로 가운데",
                            "세로 가운데",
                        ]
                        .iter()
                        .enumerate()
                        {
                            if ui.button(*label).clicked() {
                                commands::align(d, &self.canvas.selection, i);
                                ui.close();
                            }
                        }
                    });
                    ui.small(match self.tool {
                        Tool::Select => "드래그 선택/이동 · 모서리 핸들로 크기 조절",
                        Tool::Add(_) => "캔버스를 클릭해 추가",
                        Tool::Connect(_) => "시작 요소 → 도착 요소 순서로 클릭 · Esc 취소",
                    });
                });
                self.canvas_size = ui.available_size();
                self.canvas.ui(ui, d, &mut self.tool);
            });
    }
    fn dialogs(&mut self, ctx: &egui::Context) {
        if self.new_dialog || self.add_dialog {
            let adding = self.add_dialog;
            egui::Window::new(if adding {
                "다이어그램 추가"
            } else {
                "새 프로젝트"
            })
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                if !adding {
                    ui.label("프로젝트 이름");
                    ui.text_edit_singleline(&mut self.new_name);
                }
                ui.label("다이어그램 이름");
                ui.text_edit_singleline(&mut self.new_diagram);
                for k in DiagramKind::ALL {
                    ui.radio_value(&mut self.new_kind, k, k.label());
                }
                ui.horizontal(|ui| {
                    if ui
                        .add_enabled(
                            !self.new_diagram.trim().is_empty()
                                && (adding || !self.new_name.trim().is_empty()),
                            egui::Button::new("만들기"),
                        )
                        .clicked()
                    {
                        if adding {
                            if let Some(p) = &mut self.project {
                                let d = Diagram::new(self.new_kind, self.new_diagram.trim());
                                self.tabs.push(d.id);
                                p.diagrams.push(d);
                                self.active = p.diagrams.len() - 1;
                                self.canvas_hidden = false;
                                self.canvas.clear();
                                self.tool = Tool::Select;
                            }
                        } else {
                            self.replace(
                                Project::new(
                                    self.new_name.trim(),
                                    self.new_kind,
                                    self.new_diagram.trim(),
                                ),
                                None,
                                false,
                            );
                        }
                        self.new_dialog = false;
                        self.add_dialog = false;
                    }
                    if ui.button("취소").clicked() {
                        self.new_dialog = false;
                        self.add_dialog = false;
                    }
                });
            });
        }
        if self.export_dialog {
            egui::Window::new("다이어그램 전체 PNG 내보내기")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label("출력 배율");
                        for scale in 1..=3 {
                            ui.radio_value(
                                &mut self.export_options.scale,
                                scale,
                                format!("{scale}x"),
                            );
                        }
                    });
                    ui.add(
                        egui::Slider::new(&mut self.export_options.margin, 0..=200).text("여백"),
                    );
                    ui.checkbox(&mut self.export_options.transparent, "투명 배경");
                    ui.horizontal(|ui| {
                        if ui.button("파일 선택 및 내보내기").clicked() {
                            if let Some(d) = self
                                .project
                                .as_ref()
                                .and_then(|p| p.diagrams.get(self.active))
                                && let Some(path) = rfd::FileDialog::new()
                                    .add_filter("PNG", &["png"])
                                    .set_file_name(format!("{}.png", d.name))
                                    .save_file()
                            {
                                let path = path.with_extension("png");
                                match scene::export_png(
                                    &scene::build(d),
                                    &path,
                                    &self.export_options,
                                ) {
                                    Ok((w, h)) => {
                                        self.status =
                                            format!("PNG 저장됨 · {w} × {h} · {}", path.display())
                                    }
                                    Err(e) => self.error = Some(e),
                                }
                            }
                            self.export_dialog = false;
                        }
                        if ui.button("취소").clicked() {
                            self.export_dialog = false;
                        }
                    });
                });
        }
        if self.pending.is_some() {
            egui::Window::new("저장하지 않은 변경")
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("프로젝트를 저장한 뒤 계속할까요?");
                    ui.horizontal(|ui| {
                        if ui.button("저장하고 계속").clicked() && self.save(false) {
                            let a = self.pending.take().unwrap();
                            self.perform(a, ctx);
                        }
                        if ui.button("변경 버리기").clicked() {
                            let a = self.pending.take().unwrap();
                            let _ = std::fs::remove_file(
                                storage::config_dir().join("recovery.umlproj"),
                            );
                            self.perform(a, ctx);
                        }
                        if ui.button("취소").clicked() {
                            self.pending = None;
                        }
                    });
                });
        }
        if let Some(message) = self.error.clone() {
            egui::Window::new("오류")
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label(message);
                    if ui.button("확인").clicked() {
                        self.error = None;
                    }
                });
        }
    }
}
impl eframe::App for UmlApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close && self.dirty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Action::Quit);
        }
        if ctx.input(|i| i.pointer.primary_pressed()) {
            self.commit();
        }
        self.suppress_history = false;
        let before = self.project.clone();
        let modal = self.pending.is_some()
            || self.new_dialog
            || self.add_dialog
            || self.export_dialog
            || self.error.is_some();
        self.blocked = modal;
        if !modal {
            self.shortcuts(ctx);
        }
        self.toolbar(ctx);
        if self.project.is_some() {
            self.editor(ctx);
        } else {
            self.home(ctx);
        }
        self.dialogs(ctx);
        if let (Some(b), Some(p)) = (before, self.project.as_ref())
            && !self.suppress_history
            && b.id == p.id
            && !b.content_eq(p)
            && self.transaction.is_none()
        {
            self.transaction = Some(b);
        }
        if !ctx.input(|i| i.pointer.primary_down()) && !ctx.wants_keyboard_input() {
            self.commit();
        }
        if self.dirty() && self.last_recovery.elapsed() > Duration::from_secs(20) {
            if let Some(p) = &self.project {
                match storage::save(&storage::config_dir().join("recovery.umlproj"), p) {
                    Ok(()) => self.status = "자동 복구 사본 저장됨 (프로젝트 저장은 별도)".into(),
                    Err(e) => self.status = e,
                }
            }
            self.last_recovery = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
    }
}
fn field(ui: &mut egui::Ui, label: &str, s: &mut String) {
    ui.label(label);
    ui.text_edit_singleline(s);
}
fn enum_combo<T: Copy + PartialEq>(
    ui: &mut egui::Ui,
    label: &str,
    current: &mut T,
    options: &[T],
    name: impl Fn(T) -> String,
) {
    egui::ComboBox::from_id_salt(label)
        .selected_text(name(*current))
        .show_ui(ui, |ui| {
            for &v in options {
                ui.selectable_value(current, v, name(v));
            }
        });
}
fn members_ui(ui: &mut egui::Ui, members: &mut Vec<Member>, method: bool) {
    let mut remove = None;
    let mut swap = None;
    for (i, m) in members.iter_mut().enumerate() {
        ui.push_id(m.id, |ui| {
            ui.group(|ui| {
                ui.horizontal(|ui| {
                    ui.strong(m.text(method));
                    if ui.small_button("↑").clicked() && i > 0 {
                        swap = Some((i, i - 1));
                    }
                    if ui.small_button("↓").clicked() {
                        swap = Some((i, i + 1));
                    }
                    if ui.small_button("×").clicked() {
                        remove = Some(i);
                    }
                });
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("visibility")
                        .selected_text(&m.visibility)
                        .show_ui(ui, |ui| {
                            for v in ["+", "-", "#", "~"] {
                                ui.selectable_value(&mut m.visibility, v.into(), v);
                            }
                        });
                    ui.add(egui::TextEdit::singleline(&mut m.name).desired_width(130.));
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
                ui.horizontal(|ui| {
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
fn sequence_add(
    ui: &mut egui::Ui,
    s: &mut Sequence,
    selection: &mut std::collections::HashSet<Id>,
) {
    if ui.button("+ 참여자").clicked() {
        let p = Participant {
            id: id(),
            name: format!("참여자 {}", s.participants.len() + 1),
            actor: false,
        };
        selection.clear();
        selection.insert(p.id);
        s.participants.push(p);
    }
    if ui
        .add_enabled(!s.participants.is_empty(), egui::Button::new("+ 메시지"))
        .clicked()
    {
        let m = Message {
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
    if ui
        .add_enabled(!s.messages.is_empty(), egui::Button::new("+ 활성 구간"))
        .clicked()
    {
        let a = Activation {
            id: id(),
            participant: s.messages[0].target,
            start: s.messages[0].id,
            end: s.messages.last().unwrap().id,
        };
        selection.clear();
        selection.insert(a.id);
        s.activations.push(a);
    }
    if ui
        .add_enabled(!s.messages.is_empty(), egui::Button::new("+ 프래그먼트"))
        .clicked()
    {
        let f = Fragment {
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
fn sequence_list(
    ui: &mut egui::Ui,
    s: &mut Sequence,
    selection: &mut std::collections::HashSet<Id>,
) {
    for (id, label) in s
        .participants
        .iter()
        .map(|p| (p.id, format!("참여자 · {}", p.name)))
        .chain(
            s.messages
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id, format!("{} · {}", i + 1, m.text))),
        )
        .chain(s.activations.iter().map(|a| (a.id, "활성 구간".into())))
        .chain(
            s.fragments
                .iter()
                .map(|f| (f.id, format!("{:?} [{}]", f.kind, f.condition))),
        )
    {
        if ui
            .selectable_label(selection.contains(&id), label)
            .clicked()
        {
            selection.clear();
            selection.insert(id);
        }
    }
}
fn properties(ui: &mut egui::Ui, d: &mut Diagram, selection: &mut std::collections::HashSet<Id>) {
    if selection.len() != 1 {
        ui.label(if selection.is_empty() {
            "요소 또는 연결선을 선택하세요".into()
        } else {
            format!("{}개 선택됨", selection.len())
        });
        return;
    }
    let selected = *selection.iter().next().unwrap();
    let boundary_before = d
        .nodes
        .iter()
        .find(|n| n.id == selected && n.kind == NodeKind::Boundary)
        .map(|n| n.layout);
    if let Some(n) = d.nodes.iter_mut().find(|n| n.id == selected) {
        ui.strong(n.kind.label());
        field(ui, "이름", &mut n.name);
        ui.label("위치 / 크기");
        ui.horizontal(|ui| {
            ui.add(egui::DragValue::new(&mut n.layout.pos.x).prefix("X "));
            ui.add(egui::DragValue::new(&mut n.layout.pos.y).prefix("Y "));
        });
        ui.horizontal(|ui| {
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
        if matches!(
            n.kind,
            NodeKind::Class | NodeKind::Interface | NodeKind::AbstractClass
        ) {
            enum_combo(
                ui,
                "클래스 종류",
                &mut n.kind,
                &[
                    NodeKind::Class,
                    NodeKind::Interface,
                    NodeKind::AbstractClass,
                ],
                |k| k.label().into(),
            );
            if ui.button("내용에 크기 맞춤").clicked() {
                n.fit();
            }
            let counts = (n.attributes.len(), n.methods.len());
            ui.separator();
            ui.collapsing("속성", |ui| members_ui(ui, &mut n.attributes, false));
            ui.collapsing("메서드", |ui| members_ui(ui, &mut n.methods, true));
            if counts != (n.attributes.len(), n.methods.len()) {
                n.fit();
            }
        }
    } else if let Some(r) = d.relations.iter_mut().find(|r| r.id == selected) {
        ui.strong(r.kind.label());
        field(ui, "관계 이름 / 라벨", &mut r.name);
        ui.label("시작 → 도착");
        for (label, id) in [("시작", &mut r.source), ("도착", &mut r.target)] {
            egui::ComboBox::from_id_salt(label)
                .selected_text(format!(
                    "{label}: {}",
                    d.nodes
                        .iter()
                        .find(|n| n.id == *id)
                        .map(|n| n.name.as_str())
                        .unwrap_or("?")
                ))
                .show_ui(ui, |ui| {
                    for n in &d.nodes {
                        ui.selectable_value(id, n.id, &n.name);
                    }
                });
        }
        ui.small("일반화/구현의 도착은 상위 타입, 집합/합성의 시작은 전체입니다.");
        ui.checkbox(&mut r.navigable, "연관 탐색 방향 →");
        field(ui, "시작 역할", &mut r.source_role);
        field(ui, "시작 다중성", &mut r.source_multiplicity);
        field(ui, "도착 역할", &mut r.target_role);
        field(ui, "도착 다중성", &mut r.target_multiplicity);
        let mut manual = r.bend.is_some();
        if ui.checkbox(&mut manual, "직교 경로 수동 조절").changed() {
            r.bend = if manual { Some(300.) } else { None };
        }
        if let Some(bend) = &mut r.bend {
            ui.add(egui::DragValue::new(bend).prefix("중간 축 좌표 "));
        }
    } else if d.kind == DiagramKind::Sequence {
        let s = &mut d.sequence;
        if let Some(i) = s.participants.iter().position(|p| p.id == selected) {
            let p = &mut s.participants[i];
            field(ui, "참여자 이름", &mut p.name);
            ui.checkbox(&mut p.actor, "액터 형태");
            let mut swap = None;
            ui.horizontal(|ui| {
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
            field(ui, "메시지", &mut m.text);
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
            ui.horizontal(|ui| {
                if ui.button("↑").clicked() && i > 0 {
                    swap = Some(i - 1);
                }
                if ui.button("↓").clicked() {
                    swap = Some(i + 1);
                }
            });
            ui.horizontal(|ui| {
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
            if child.kind == NodeKind::UseCase && old.contains(child.layout.center()) {
                child.layout.pos.x += delta.x;
                child.layout.pos.y += delta.y;
            }
        }
    }
    ui.separator();
    if ui.button("선택 삭제").clicked() {
        d.delete(selection);
        selection.clear();
    }
}

pub fn example_project() -> Project {
    let mut p = Project::new("정렬 프로그램", DiagramKind::UseCase, "정렬 기능");
    let d = &mut p.diagrams[0];
    let mut boundary = Node::new(NodeKind::Boundary, Point::new(230., 20.));
    boundary.name = "정렬 프로그램".into();
    boundary.layout.size = Point::new(580., 390.);
    let mut actor = Node::new(NodeKind::Actor, Point::new(40., 140.));
    actor.name = "사용자".into();
    let mut sort = Node::new(NodeKind::UseCase, Point::new(300., 100.));
    sort.name = "정렬 실행".into();
    let mut policy = Node::new(NodeKind::UseCase, Point::new(540., 240.));
    policy.name = "정렬 정책 선택".into();
    d.relations = vec![
        Relation::new(actor.id, sort.id, RelationKind::Association),
        Relation::new(sort.id, policy.id, RelationKind::Include),
    ];
    d.nodes = vec![boundary, actor, sort, policy];
    let mut d = Diagram::new(DiagramKind::Class, "정렬 프로그램 구조");
    let mut service = Node::new(NodeKind::Class, Point::new(40., 80.));
    service.name = "SortService".into();
    service.attributes.push(Member {
        visibility: "-".into(),
        name: "policy".into(),
        type_name: "SortPolicy".into(),
        ..Default::default()
    });
    service.methods.push(Member {
        name: "sort".into(),
        parameters: "values: List<int>".into(),
        ..Default::default()
    });
    service.fit();
    let mut policy = Node::new(NodeKind::Interface, Point::new(560., 80.));
    policy.name = "SortPolicy".into();
    policy.methods.push(Member {
        name: "sort".into(),
        parameters: "values: List<int>".into(),
        is_abstract: true,
        ..Default::default()
    });
    policy.fit();
    let mut bubble = Node::new(NodeKind::Class, Point::new(560., 370.));
    bubble.name = "BubbleSort".into();
    bubble.methods.push(Member {
        name: "sort".into(),
        parameters: "values: List<int>".into(),
        ..Default::default()
    });
    bubble.fit();
    let mut r = Relation::new(service.id, policy.id, RelationKind::Association);
    r.navigable = true;
    r.target_multiplicity = "1".into();
    r.target_role = "policy".into();
    d.relations = vec![
        r,
        Relation::new(bubble.id, policy.id, RelationKind::Realization),
    ];
    d.nodes = vec![service, policy, bubble];
    p.diagrams.push(d);
    let mut d = Diagram::new(DiagramKind::Sequence, "정렬 실행");
    let user = Participant {
        id: id(),
        name: "사용자".into(),
        actor: true,
    };
    let svc = Participant {
        id: id(),
        name: "service: SortService".into(),
        actor: false,
    };
    let bubble = Participant {
        id: id(),
        name: "policy: BubbleSort".into(),
        actor: false,
    };
    let m1 = Message {
        id: id(),
        source: user.id,
        target: svc.id,
        kind: MessageKind::Sync,
        text: "sort(values)".into(),
    };
    let m2 = Message {
        id: id(),
        source: svc.id,
        target: bubble.id,
        kind: MessageKind::Sync,
        text: "sort(values)".into(),
    };
    let m3 = Message {
        id: id(),
        source: bubble.id,
        target: bubble.id,
        kind: MessageKind::Sync,
        text: "인접 원소 비교 및 교환".into(),
    };
    let m4 = Message {
        id: id(),
        source: bubble.id,
        target: svc.id,
        kind: MessageKind::Return,
        text: "정렬 완료".into(),
    };
    let m5 = Message {
        id: id(),
        source: svc.id,
        target: user.id,
        kind: MessageKind::Return,
        text: "정렬된 결과".into(),
    };
    d.sequence.activations = vec![
        Activation {
            id: id(),
            participant: svc.id,
            start: m1.id,
            end: m5.id,
        },
        Activation {
            id: id(),
            participant: bubble.id,
            start: m2.id,
            end: m4.id,
        },
    ];
    d.sequence.fragments.push(Fragment {
        id: id(),
        kind: FragmentKind::Loop,
        condition: "교환이 발생하는 동안".into(),
        else_condition: String::new(),
        start: m3.id,
        end: m3.id,
        split: None,
    });
    d.sequence.participants = vec![user, svc, bubble];
    d.sequence.messages = vec![m1, m2, m3, m4, m5];
    p.diagrams.push(d);
    p
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::App;
    fn app(ctx: &egui::Context) -> UmlApp {
        let mut app = UmlApp::new(&eframe::CreationContext::_new_kittest(ctx.clone()));
        app.replace(example_project(), None, true);
        app
    }
    fn key(key: Key, command: bool) -> egui::RawInput {
        let modifiers = egui::Modifiers {
            command,
            ctrl: command,
            ..Default::default()
        };
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1440., 900.),
            )),
            modifiers,
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        }
    }
    #[test]
    fn app_undo_does_not_record_itself_and_redo_survives_next_frame() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        let before = app.project.clone().unwrap();
        let p = app.project.as_mut().unwrap();
        p.name = "수정".into();
        app.history.record(before.clone(), p);
        let mut frame = eframe::Frame::_new_kittest();
        let _ = ctx.run(key(Key::Z, true), |ctx| app.update(ctx, &mut frame));
        assert!(app.project.as_ref().unwrap().content_eq(&before));
        assert!(app.history.can_redo());
        let _ = ctx.run(egui::RawInput::default(), |ctx| app.update(ctx, &mut frame));
        assert!(app.history.can_redo());
        let _ = ctx.run(key(Key::Y, true), |ctx| app.update(ctx, &mut frame));
        assert_eq!(app.project.as_ref().unwrap().name, "수정");
    }
    #[test]
    fn text_focus_blocks_canvas_delete_and_duplicate_shortcuts() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        let node = app.project.as_ref().unwrap().diagrams[0].nodes[1].id;
        app.canvas.selection.insert(node);
        let before = app.project.clone().unwrap();
        let mut text = "편집 중".to_string();
        let _ = ctx.run(key(Key::Delete, false), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("focus-test"))
                    .request_focus();
            });
            assert!(ctx.wants_keyboard_input());
            app.shortcuts(ctx);
        });
        assert_eq!(app.project.as_ref().unwrap(), &before);
        let _ = ctx.run(key(Key::D, true), |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.add(egui::TextEdit::singleline(&mut text).id_salt("focus-test"))
                    .request_focus();
            });
            app.shortcuts(ctx);
        });
        assert_eq!(app.project.as_ref().unwrap(), &before);
        ctx.memory_mut(|m| m.surrender_focus(egui::Id::new("focus-test")));
        // Drop any text focus, then the canvas command must delete the selected actor.
        ctx.memory_mut(|m| {
            if let Some(id) = m.focused() {
                m.surrender_focus(id);
            }
        });
        let _ = ctx.run(key(Key::Delete, false), |ctx| app.shortcuts(ctx));
        assert_eq!(
            app.project.as_ref().unwrap().diagrams[0].nodes.len(),
            before.diagrams[0].nodes.len() - 1
        );
    }
    #[test]
    fn replacing_dirty_project_requires_a_decision() {
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        app.project.as_mut().unwrap().name = "미저장".into();
        app.request(Action::Home, &ctx);
        assert!(matches!(app.pending, Some(Action::Home)));
        assert!(app.project.is_some());
        app.pending = None;
        assert!(app.dirty());
    }
}
