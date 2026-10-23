use crate::properties::{self, Catalog, PropertiesState, sequence_add, sequence_list};
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
    properties_state: PropertiesState,
    show_project: bool,
    show_properties: bool,
    show_diagnostics: bool,
    search: String,
    export_svg: bool,
    preview: Option<egui::TextureHandle>,
    preview_key: String,
    compact: bool,
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
            properties_state: PropertiesState::default(),
            show_project: true,
            show_properties: true,
            show_diagnostics: true,
            search: String::new(),
            export_svg: false,
            preview: None,
            preview_key: String::new(),
            compact: false,
        }
    }
    pub fn inspect_element(&mut self, id: Id) {
        if let Some((index, diagram)) = self.project.as_ref().and_then(|p| {
            p.diagrams
                .iter()
                .enumerate()
                .find(|(_, d)| {
                    d.nodes.iter().any(|n| n.id == id)
                        || d.relations.iter().any(|r| r.id == id)
                        || d.sequence.participants.iter().any(|p| p.id == id)
                        || d.sequence.messages.iter().any(|m| m.id == id)
                        || d.sequence.fragments.iter().any(|f| f.id == id)
                })
                .map(|(i, d)| (i, d.id))
        }) {
            self.active = index;
            self.canvas_hidden = false;
            self.show_properties = true;
            self.canvas.selection.clear();
            self.canvas.selection.insert(id);
            if !self.tabs.contains(&diagram) {
                self.tabs.push(diagram);
            }
        }
    }
    pub fn fit_active(&mut self) {
        if let Some(d) = self
            .project
            .as_mut()
            .and_then(|p| p.diagrams.get_mut(self.active))
        {
            canvas::fit(d, self.canvas_size);
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
        self.properties_state = PropertiesState::default();
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
        self.properties_state = PropertiesState::default();
        self.commit();
        if let Some(p) = &mut self.project {
            self.history.undo(p);
            self.active = self.active.min(p.diagrams.len().saturating_sub(1));
            self.canvas.clear();
        }
    }
    fn redo(&mut self) {
        self.suppress_history = true;
        self.properties_state = PropertiesState::default();
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
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    RichText::new("UML STUDIO")
                        .strong()
                        .color(Color32::from_rgb(47, 79, 172)),
                );
                ui.separator();
                ui.menu_button("보기", |ui| {
                    ui.checkbox(&mut self.show_project, "프로젝트 패널");
                    ui.checkbox(&mut self.show_properties, "속성 패널");
                    ui.checkbox(&mut self.show_diagnostics, "진단 패널");
                });
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
                    .add_enabled(
                        has && self
                            .project
                            .as_ref()
                            .is_some_and(|p| !p.diagrams.is_empty()),
                        egui::Button::new("내보내기"),
                    )
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
            ui.horizontal_wrapped(|ui| {
                ui.small(&self.status);
                ui.separator();
                ui.small(format!("{}개 선택", self.canvas.selection.len()));
                if let Some(d) = self
                    .project
                    .as_ref()
                    .and_then(|p| p.diagrams.get(self.active))
                {
                    ui.small(format!("{:.0}%", d.view.zoom * 100.));
                }
                ui.small(match self.tool {
                    Tool::Select => "선택 · Alt+클릭 겹친 대상 전환",
                    Tool::Add(_) => "캔버스 클릭으로 추가 · Esc 취소",
                    Tool::Connect(_) => "시작 → 도착 클릭 또는 드래그 · Esc 취소",
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
                ui.label("유스케이스 → 시퀀스 → 클래스 순서로 살펴보거나, 필요한 종류부터 자유롭게 시작하세요.");
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
        let catalog = self
            .project
            .as_ref()
            .map(Catalog::from_project)
            .unwrap_or_default();
        if let Some(id) = self.canvas.edit_name.take() {
            self.properties_state.focus_name = Some(id);
        }
        let mut select = None;
        let mut remove = None;
        let mut duplicate = None;
        let issues = self
            .project
            .as_ref()
            .map(|p| p.diagnostics())
            .unwrap_or_default();
        if self.show_diagnostics && !issues.is_empty() {
            egui::TopBottomPanel::bottom("diagnostics")
                .resizable(true)
                .default_height(90.)
                .show(ctx, |ui| {
                    ui.strong(format!(
                        "진단 {}개 · 정상 저장 전 수정이 필요합니다",
                        issues.len()
                    ));
                    egui::ScrollArea::vertical().show(ui, |ui| {
                        for issue in &issues {
                            if ui.selectable_label(false, &issue.message).clicked()
                                && let Some(i) = self.project.as_ref().and_then(|p| {
                                    p.diagrams.iter().position(|d| d.id == issue.diagram)
                                })
                            {
                                self.active = i;
                                self.canvas_hidden = false;
                                self.canvas.selection.clear();
                                self.canvas.selection.insert(issue.element);
                                self.show_properties = true;
                            }
                        }
                    });
                });
        }
        let search = self.search.to_lowercase();
        if self.show_project {
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
                        ui.add(
                            egui::TextEdit::singleline(&mut self.search)
                                .hint_text("다이어그램·요소 검색"),
                        );
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.strong("다이어그램");
                            if ui.small_button("+ 추가").clicked() {
                                self.add_dialog = true;
                            }
                        });
                        egui::ScrollArea::vertical()
                            .id_salt("diagram-list")
                            .max_height(180.)
                            .show(ui, |ui| {
                                for (i, d) in p.diagrams.iter().enumerate() {
                                    if !search.is_empty()
                                        && !d.name.to_lowercase().contains(&search)
                                        && !d
                                            .nodes
                                            .iter()
                                            .any(|n| n.name.to_lowercase().contains(&search))
                                        && !d
                                            .sequence
                                            .participants
                                            .iter()
                                            .any(|p| p.name.to_lowercase().contains(&search))
                                    {
                                        continue;
                                    }
                                    let row = ui.add(
                                        egui::Button::new(format!(
                                            "{} · {}",
                                            d.kind.label(),
                                            d.name
                                        ))
                                        .selected(i == self.active)
                                        .truncate(),
                                    );
                                    if row.clicked() {
                                        select = Some(i);
                                    }
                                    row.clone().on_hover_text(&d.name);
                                    row.context_menu(|ui| {
                                        if ui.button("이름 변경").clicked() {
                                            select = Some(i);
                                            ui.memory_mut(|m| {
                                                m.request_focus(egui::Id::new((
                                                    "diagram-name",
                                                    d.id,
                                                )))
                                            });
                                            ui.close();
                                        }
                                        if ui.button("다이어그램 복제").clicked() {
                                            duplicate = Some(i);
                                            ui.close();
                                        }
                                        if ui.button("다이어그램 삭제").clicked() {
                                            remove = Some(i);
                                            ui.close();
                                        }
                                    });
                                }
                            });
                        if let Some(d) = p.diagrams.get_mut(self.active) {
                            ui.separator();
                            ui.label("다이어그램 이름");
                            ui.add(
                                egui::TextEdit::singleline(&mut d.name)
                                    .id(egui::Id::new(("diagram-name", d.id))),
                            );
                            ui.add_space(15.);
                            ui.label("요소 목록");
                            egui::ScrollArea::vertical()
                                .id_salt("element-list")
                                .show(ui, |ui| {
                                    let mut focus = None;
                                    for n in &d.nodes {
                                        if !n.name.to_lowercase().contains(&search) {
                                            continue;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.canvas.selection.contains(&n.id),
                                                &n.name,
                                            )
                                            .clicked()
                                        {
                                            self.canvas.selection.clear();
                                            self.canvas.selection.insert(n.id);
                                            focus = Some(n.id);
                                        }
                                    }
                                    for r in &d.relations {
                                        if !format!("{} {}", r.kind.label(), r.name)
                                            .to_lowercase()
                                            .contains(&search)
                                        {
                                            continue;
                                        }
                                        if ui
                                            .selectable_label(
                                                self.canvas.selection.contains(&r.id),
                                                format!("관계 · {} {}", r.kind.label(), r.name),
                                            )
                                            .clicked()
                                        {
                                            self.canvas.selection.clear();
                                            self.canvas.selection.insert(r.id);
                                            focus = Some(r.id);
                                        }
                                    }
                                    if d.kind == DiagramKind::Sequence
                                        && let Some(id) = sequence_list(
                                            ui,
                                            &mut d.sequence,
                                            &mut self.canvas.selection,
                                            &search,
                                        )
                                    {
                                        focus = Some(id);
                                    }
                                    if let Some(id) = focus
                                        && let Some(rect) =
                                            scene::build(d).selection_boxes().get(&id)
                                    {
                                        d.view.pan = Point::new(
                                            self.canvas_size.x / 2. - rect.center().x * d.view.zoom,
                                            self.canvas_size.y / 2. - rect.center().y * d.view.zoom,
                                        );
                                    }
                                });
                        }
                    }
                });
        }
        if let Some(i) = duplicate
            && let Some(p) = &mut self.project
        {
            let d = commands::duplicate_diagram(&p.diagrams[i]);
            self.tabs.push(d.id);
            p.diagrams.push(d);
            self.active = p.diagrams.len() - 1;
            self.canvas_hidden = false;
        }
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
        if self.show_properties {
            // Keep room for the canvas, including when restoring an oversized panel.
            let max_width = (ctx.available_rect().width() - 320.).clamp(180., 420.);
            egui::SidePanel::right("properties")
                .default_width(310.)
                .width_range(260_f32.min(max_width)..=max_width)
                .show(ctx, |ui| {
                    if self.blocked {
                        ui.disable();
                    }
                    ui.heading("속성");
                    // A vertical-only ScrollArea grows horizontally to fit its content,
                    // feeding that larger width back into the SidePanel every frame.
                    egui::ScrollArea::both()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                            if let Some(d) = self
                                .project
                                .as_mut()
                                .and_then(|p| p.diagrams.get_mut(self.active))
                            {
                                properties::properties(
                                    ui,
                                    d,
                                    &mut self.canvas.selection,
                                    &catalog,
                                    &mut self.properties_state,
                                );
                            }
                        });
                });
        }
        egui::CentralPanel::default()
            .frame(egui::Frame::new().fill(Color32::from_rgb(247, 249, 252)))
            .show(ctx, |ui| {
                if self.blocked {
                    ui.disable();
                }
                let mut tab_select = None;
                let mut tab_close = None;
                if let Some(p) = &self.project {
                    egui::ScrollArea::horizontal()
                        .id_salt("tabs-scroll")
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                for id in &self.tabs {
                                    if let Some((i, d)) =
                                        p.diagrams.iter().enumerate().find(|(_, d)| d.id == *id)
                                    {
                                        if ui
                                            .add_sized(
                                                [160., 30.],
                                                egui::Button::new(&d.name)
                                                    .selected(i == self.active)
                                                    .truncate(),
                                            )
                                            .clicked()
                                        {
                                            tab_select = Some(i);
                                        }
                                        if ui
                                            .small_button("닫기")
                                            .on_hover_text("탭 닫기 · 다이어그램은 유지됩니다")
                                            .clicked()
                                        {
                                            tab_close = Some(*id);
                                        }
                                    }
                                }
                            });
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
                            NodeKind::Note,
                            NodeKind::Enumeration,
                            NodeKind::Package,
                        ],
                        DiagramKind::UseCase => {
                            vec![
                                NodeKind::Actor,
                                NodeKind::UseCase,
                                NodeKind::Boundary,
                                NodeKind::Note,
                            ]
                        }
                        DiagramKind::Sequence => vec![],
                    };
                    if !kinds.is_empty() {
                        ui.menu_button("요소 추가", |ui| {
                            for k in kinds {
                                crate::palette::node_tool(ui, &mut self.tool, k);
                            }
                        });
                    }
                    if let Tool::Add(kind) = self.tool {
                        ui.label(format!("현재 도구: {}", kind.label()));
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
                                                | RelationKind::Comment
                                        ))
                                {
                                    continue;
                                }
                                if ui
                                    .selectable_label(self.tool == Tool::Connect(k), k.label())
                                    .on_hover_text(match (d.kind, k) {
                                        (DiagramKind::UseCase, RelationKind::Association) =>
                                            "액터 ↔ 유스케이스를 연결합니다. 유스케이스끼리는 include / extend / 일반화를 선택하세요.",
                                        (_, RelationKind::Include) => "포함하는 유스케이스 → 포함되는 유스케이스",
                                        (_, RelationKind::Extend) => "확장하는 유스케이스 → 기본 유스케이스",
                                        (_, RelationKind::Generalization) => "하위 요소 → 상위 요소 (액터끼리, 유스케이스끼리, 클래스끼리, 인터페이스끼리)",
                                        (_, RelationKind::Realization) => "구현 클래스 → 인터페이스",
                                        (_, RelationKind::Aggregation | RelationKind::Composition) => "전체 → 부분 (다이아몬드는 시작 요소 쪽에 표시됩니다)",
                                        (_, RelationKind::Comment) => "한쪽 대상을 노트로 선택하세요",
                                        _ => "시작 요소 → 도착 요소를 클릭하거나 드래그하세요",
                                    })
                                    .clicked()
                                {
                                    self.tool = Tool::Connect(k);
                                    self.canvas.connection = None;
                                    ui.close();
                                }
                            }
                        });
                        if let Tool::Connect(kind) = self.tool {
                            ui.label(format!(
                                "{} · {}",
                                kind.label(),
                                if self.canvas.connection.is_some() {
                                    "도착 요소를 선택하세요"
                                } else {
                                    "시작 요소를 선택하세요"
                                },
                            ));
                            if ui.button("연결 취소").clicked() {
                                self.tool = Tool::Select;
                                self.canvas.connection = None;
                            }
                        }
                    } else {
                        ui.menu_button("시퀀스 추가", |ui| {
                            sequence_add(ui, &mut d.sequence, &mut self.canvas.selection)
                        });
                        let selected: Vec<_> = d
                            .sequence
                            .participants
                            .iter()
                            .filter(|p| self.canvas.selection.contains(&p.id))
                            .map(|p| p.id)
                            .collect();
                        if selected.len() == 2 && ui.button("선택 참여자로 메시지").clicked()
                        {
                            let m = Message {
                                id: id(),
                                source: selected[0],
                                target: selected[1],
                                kind: MessageKind::Sync,
                                text: "호출()".into(),
                                method_ref: None,
                            };
                            self.canvas.selection.clear();
                            self.canvas.selection.insert(m.id);
                            d.sequence.messages.push(m);
                        }
                        if ui
                            .checkbox(&mut d.sequence.auto_activate, "활성 구간 자동 생성")
                            .changed()
                        {
                            d.sequence.regenerate_activations();
                        }
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    ui.checkbox(&mut self.canvas.grid, "그리드");
                    ui.checkbox(&mut self.canvas.snap, "맞춤");
                    if ui.button("화면 맞춤").clicked() {
                        canvas::fit(d, self.canvas_size);
                    }
                    egui::ComboBox::from_id_salt("zoom")
                        .selected_text(format!("{:.0}%", d.view.zoom * 100.))
                        .show_ui(ui, |ui| {
                            for z in [0.25, 0.5, 0.75, 1., 1.25, 1.5, 2.] {
                                if ui
                                    .selectable_label(d.view.zoom == z, format!("{:.0}%", z * 100.))
                                    .clicked()
                                {
                                    let world = Point::new(
                                        (self.canvas_size.x / 2. - d.view.pan.x) / d.view.zoom,
                                        (self.canvas_size.y / 2. - d.view.pan.y) / d.view.zoom,
                                    );
                                    d.view.zoom = z;
                                    d.view.pan = Point::new(
                                        self.canvas_size.x / 2. - world.x * z,
                                        self.canvas_size.y / 2. - world.y * z,
                                    );
                                }
                            }
                        });
                    ui.menu_button("선택 편집", |ui| {
                        ui.checkbox(&mut self.canvas.guides, "정렬 가이드");
                        for (label, horizontal) in
                            [("가로 균등 간격", true), ("세로 균등 간격", false)]
                        {
                            if ui.button(label).clicked() {
                                commands::distribute(d, &self.canvas.selection, horizontal);
                                ui.close();
                            }
                        }
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
                        Tool::Connect(_) => "시작 → 도착 클릭 또는 드래그 · Esc 취소",
                    });
                });
                self.canvas_size = ui.available_size();
                self.canvas.ui(ui, d, &mut self.tool);
                if let Some(action) = self.canvas.context_action.take() {
                    match action {
                        canvas::ContextAction::Delete => {
                            d.delete(&self.canvas.selection);
                            self.canvas.selection.clear();
                        }
                        canvas::ContextAction::Copy => {
                            self.clipboard = Clipboard::copy(d, &self.canvas.selection)
                        }
                        canvas::ContextAction::Paste => {
                            self.canvas.selection = self.clipboard.paste(d)
                        }
                        canvas::ContextAction::Duplicate => {
                            self.clipboard = Clipboard::copy(d, &self.canvas.selection);
                            self.canvas.selection = self.clipboard.paste(d);
                        }
                    }
                }
                if let Some(notice) = self.canvas.notice.take() {
                    self.status = notice;
                }
                if self.canvas.edit_name.is_some() {
                    self.show_properties = true;
                }
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
            egui::Window::new("다이어그램 전체 내보내기")
                .default_width(620.)
                .collapsible(false)
                .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut self.export_svg, false, "PNG 이미지");
                        ui.radio_value(&mut self.export_svg, true, "SVG 벡터");
                    });
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
                    let mut export_ready = false;
                    if let Some(d) = self
                        .project
                        .as_ref()
                        .and_then(|p| p.diagrams.get(self.active))
                    {
                        let drawing = scene::build(d);
                        match scene::svg(&drawing, &self.export_options) {
                            Ok((svg, w, h)) => {
                                ui.label(format!("예상 출력: {w} × {h} px"));
                                export_ready = true;
                                if self.preview_key != svg {
                                    match crate::export::preview(&svg, w, h) {
                                        Ok(image) => {
                                            self.preview = Some(ctx.load_texture(
                                                "export-preview",
                                                image,
                                                egui::TextureOptions::LINEAR,
                                            ));
                                            self.preview_key = svg;
                                        }
                                        Err(e) => {
                                            ui.colored_label(Color32::DARK_RED, e);
                                        }
                                    }
                                }
                                if let Some(texture) = &self.preview {
                                    ui.image(texture);
                                }
                            }
                            Err(e) => {
                                ui.colored_label(Color32::DARK_RED, e);
                            }
                        }
                    }
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(export_ready, egui::Button::new("파일 선택 및 내보내기"))
                            .clicked()
                        {
                            let extension = if self.export_svg { "svg" } else { "png" };
                            if let Some(d) = self
                                .project
                                .as_ref()
                                .and_then(|p| p.diagrams.get(self.active))
                                && let Some(path) = rfd::FileDialog::new()
                                    .add_filter(extension, &[extension])
                                    .set_file_name(format!("{}.{}", d.name, extension))
                                    .save_file()
                            {
                                let path = path.with_extension(extension);
                                let drawing = scene::build(d);
                                let result = if self.export_svg {
                                    crate::export::svg(&drawing, &path, &self.export_options)
                                } else {
                                    scene::export_png(&drawing, &path, &self.export_options)
                                };
                                match result {
                                    Ok((w, h)) => {
                                        self.status = format!(
                                            "내보내기 완료 · {w} × {h} · {}",
                                            path.display()
                                        );
                                        self.export_dialog = false;
                                    }
                                    Err(e) => self.error = Some(e),
                                }
                            }
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
        let is_compact = ctx.content_rect().width() < 1000.;
        if is_compact && !self.compact {
            self.show_project = false;
            self.show_properties = false;
        }
        self.compact = is_compact;
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
        if let Some(p) = &mut self.project {
            p.synchronize_links();
            for d in &mut p.diagrams {
                for n in &mut d.nodes {
                    n.reflow();
                }
                let old = before
                    .as_ref()
                    .and_then(|p| p.diagrams.iter().find(|old| old.id == d.id));
                if d.sequence.auto_activate
                    && old.is_none_or(|old| {
                        old.sequence.messages != d.sequence.messages
                            || old.sequence.auto_activate != d.sequence.auto_activate
                    })
                {
                    d.sequence.regenerate_activations();
                }
            }
        }
        if let Some(id) = self.properties_state.open_diagram.take()
            && let Some(i) = self
                .project
                .as_ref()
                .and_then(|p| p.diagrams.iter().position(|d| d.id == id))
        {
            self.active = i;
            self.canvas_hidden = false;
            self.canvas.clear();
            if !self.tabs.contains(&id) {
                self.tabs.push(id);
            }
        }
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
                match storage::save_recovery(&storage::config_dir().join("recovery.umlproj"), p) {
                    Ok(()) => self.status = "자동 복구 사본 저장됨 (프로젝트 저장은 별도)".into(),
                    Err(e) => self.status = e,
                }
            }
            self.last_recovery = Instant::now();
        }
        ctx.request_repaint_after(Duration::from_secs(1));
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
        type_ref: None,
        id: id(),
        name: "사용자".into(),
        actor: true,
    };
    let svc = Participant {
        type_ref: None,
        id: id(),
        name: "service: SortService".into(),
        actor: false,
    };
    let bubble = Participant {
        type_ref: None,
        id: id(),
        name: "policy: BubbleSort".into(),
        actor: false,
    };
    let m1 = Message {
        method_ref: None,
        id: id(),
        source: user.id,
        target: svc.id,
        kind: MessageKind::Sync,
        text: "sort(values)".into(),
    };
    let m2 = Message {
        method_ref: None,
        id: id(),
        source: svc.id,
        target: bubble.id,
        kind: MessageKind::Sync,
        text: "sort(values)".into(),
    };
    let m3 = Message {
        method_ref: None,
        id: id(),
        source: bubble.id,
        target: bubble.id,
        kind: MessageKind::Sync,
        text: "인접 원소 비교 및 교환".into(),
    };
    let m4 = Message {
        method_ref: None,
        id: id(),
        source: bubble.id,
        target: svc.id,
        kind: MessageKind::Return,
        text: "정렬 완료".into(),
    };
    let m5 = Message {
        method_ref: None,
        id: id(),
        source: svc.id,
        target: user.id,
        kind: MessageKind::Return,
        text: "정렬된 결과".into(),
    };
    d.sequence.activations = vec![
        Activation {
            automatic: false,
            id: id(),
            participant: svc.id,
            start: m1.id,
            end: m5.id,
        },
        Activation {
            automatic: false,
            id: id(),
            participant: bubble.id,
            start: m2.id,
            end: m4.id,
        },
    ];
    d.sequence.fragments.push(Fragment {
        participant_start: None,
        participant_end: None,
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
    let boundary = p.diagrams[0].nodes[0].id;
    for n in &mut p.diagrams[0].nodes {
        if n.kind == NodeKind::UseCase {
            n.boundary = Some(boundary);
        }
    }
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
    fn usecase_relation_menu_and_canvas_clicks_create_one_undoable_relation() {
        fn run(
            ctx: &egui::Context,
            app: &mut UmlApp,
            frame: &mut eframe::Frame,
            events: Vec<egui::Event>,
        ) -> egui::FullOutput {
            ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(1440., 900.),
                    )),
                    events,
                    ..Default::default()
                },
                |ctx| app.update(ctx, frame),
            )
        }
        fn text_center(output: &egui::FullOutput, label: &str) -> egui::Pos2 {
            fn find(shape: &egui::Shape, label: &str) -> Option<egui::Pos2> {
                match shape {
                    egui::Shape::Text(t) if t.galley.job.text == label => {
                        Some(t.pos + t.galley.size() / 2.)
                    }
                    egui::Shape::Vec(shapes) => shapes.iter().find_map(|s| find(s, label)),
                    _ => None,
                }
            }
            output
                .shapes
                .iter()
                .find_map(|s| find(&s.shape, label))
                .expect(label)
        }
        fn click(
            ctx: &egui::Context,
            app: &mut UmlApp,
            frame: &mut eframe::Frame,
            pos: egui::Pos2,
        ) -> egui::FullOutput {
            run(ctx, app, frame, vec![egui::Event::PointerMoved(pos)]);
            run(
                ctx,
                app,
                frame,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                }],
            );
            run(
                ctx,
                app,
                frame,
                vec![egui::Event::PointerButton {
                    pos,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                }],
            )
        }
        fn origin(ctx: &egui::Context, app: &UmlApp) -> egui::Pos2 {
            let bottom = egui::containers::panel::PanelState::load(ctx, egui::Id::new("status"))
                .unwrap()
                .rect
                .top();
            egui::pos2(0., bottom - app.canvas_size.y)
        }
        let ctx = egui::Context::default();
        let mut app = app(&ctx);
        let mut p = Project::new("test", DiagramKind::UseCase, "test");
        p.diagrams[0].nodes = vec![
            Node::new(NodeKind::Actor, Point::new(100., 100.)),
            Node::new(NodeKind::UseCase, Point::new(500., 200.)),
        ];
        app.replace(p, None, true);
        app.show_project = false;
        app.show_properties = false;
        let mut frame = eframe::Frame::_new_kittest();
        let output = run(&ctx, &mut app, &mut frame, vec![]);
        click(
            &ctx,
            &mut app,
            &mut frame,
            text_center(&output, "관계 연결"),
        );
        let output = run(&ctx, &mut app, &mut frame, vec![]);
        let pos = text_center(&output, "연관");
        click(&ctx, &mut app, &mut frame, pos);
        assert!(matches!(app.tool, Tool::Connect(RelationKind::Association)));
        run(&ctx, &mut app, &mut frame, vec![]);
        let d = &app.project.as_ref().unwrap().diagrams[0];
        let source = d.nodes[0].id;
        let pos = canvas::world_to_screen(Point::new(104., 135.), origin(&ctx, &app), d.view);
        click(&ctx, &mut app, &mut frame, pos);
        assert_eq!(app.canvas.connection, Some(source));
        let d = &app.project.as_ref().unwrap().diagrams[0];
        let pos = canvas::world_to_screen(d.nodes[1].layout.center(), origin(&ctx, &app), d.view);
        click(&ctx, &mut app, &mut frame, pos);
        assert_eq!(app.project.as_ref().unwrap().diagrams[0].relations.len(), 1);
        assert!(app.history.can_undo());
        let _ = ctx.run(key(Key::Z, true), |ctx| app.update(ctx, &mut frame));
        assert!(
            app.project.as_ref().unwrap().diagrams[0]
                .relations
                .is_empty()
        );
        let _ = ctx.run(key(Key::Y, true), |ctx| app.update(ctx, &mut frame));
        assert_eq!(app.project.as_ref().unwrap().diagrams[0].relations.len(), 1);
    }
    #[test]
    fn properties_panel_stays_bounded_across_frames_and_window_sizes() {
        for (width, stored_width) in [1440., 1000., 820.]
            .into_iter()
            .flat_map(|width| [310., 900.].map(|stored| (width, stored)))
        {
            let ctx = egui::Context::default();
            let mut app = app(&ctx);
            app.replace(crate::demo::review_project(), None, true);
            for diagram in &mut app.project.as_mut().unwrap().diagrams {
                diagram.name = "아주 긴 시퀀스 다이어그램 이름 ".repeat(40);
            }
            let mut frame = eframe::Frame::_new_kittest();
            let input = || egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(width, 900.),
                )),
                ..Default::default()
            };
            let _ = ctx.run(input(), |ctx| app.update(ctx, &mut frame));
            app.show_project = false;
            // Recover even when an earlier version persisted an oversized inspector.
            ctx.data_mut(|data| {
                data.insert_persisted(
                    egui::Id::new("properties"),
                    egui::containers::panel::PanelState {
                        rect: egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(stored_width, 900.),
                        ),
                    },
                );
            });
            let nodes: Vec<_> = app
                .project
                .as_ref()
                .unwrap()
                .diagrams
                .iter()
                .flat_map(|d| d.nodes.iter().map(|n| n.id))
                .collect();
            for node in nodes {
                app.inspect_element(node);
                for _ in 0..12 {
                    let _ = ctx.run(input(), |ctx| app.update(ctx, &mut frame));
                    let rect = egui::containers::panel::PanelState::load(
                        &ctx,
                        egui::Id::new("properties"),
                    )
                    .unwrap()
                    .rect;
                    assert!(rect.width() <= 421., "inspector expanded to {rect:?}");
                    assert!(rect.right() <= width + 1., "inspector overflowed: {rect:?}");
                    assert!(rect.left() >= 320., "canvas was covered: {rect:?}");
                }
            }
        }
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
