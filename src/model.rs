use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;
pub type Id = Uuid;
pub fn id() -> Id {
    Uuid::new_v4()
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}
impl Point {
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Layout {
    pub pos: Point,
    pub size: Point,
}
impl Layout {
    pub fn new(x: f32, y: f32, w: f32, h: f32) -> Self {
        Self {
            pos: Point::new(x, y),
            size: Point::new(w, h),
        }
    }
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.pos.x
            && p.y >= self.pos.y
            && p.x <= self.pos.x + self.size.x
            && p.y <= self.pos.y + self.size.y
    }
    pub fn center(&self) -> Point {
        Point::new(self.pos.x + self.size.x / 2., self.pos.y + self.size.y / 2.)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiagramKind {
    Class,
    UseCase,
    Sequence,
}
impl DiagramKind {
    pub const ALL: [Self; 3] = [Self::Class, Self::UseCase, Self::Sequence];
    pub fn label(self) -> &'static str {
        match self {
            Self::Class => "클래스",
            Self::UseCase => "유스케이스",
            Self::Sequence => "시퀀스",
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeKind {
    Class,
    Interface,
    AbstractClass,
    Actor,
    UseCase,
    Boundary,
    Note,
    Enumeration,
    Package,
}
impl NodeKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Class => "클래스",
            Self::Interface => "인터페이스",
            Self::AbstractClass => "추상 클래스",
            Self::Actor => "액터",
            Self::UseCase => "유스케이스",
            Self::Boundary => "시스템 경계",
            Self::Note => "노트",
            Self::Enumeration => "열거형",
            Self::Package => "패키지",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id: Id,
    pub visibility: String,
    pub name: String,
    pub type_name: String,
    pub value: String,
    pub parameters: String,
    pub is_static: bool,
    pub is_abstract: bool,
}
impl Default for Member {
    fn default() -> Self {
        Self {
            id: id(),
            visibility: "+".into(),
            name: "member".into(),
            type_name: "void".into(),
            value: String::new(),
            parameters: String::new(),
            is_static: false,
            is_abstract: false,
        }
    }
}
impl Member {
    pub fn parse(input: &str, method: bool) -> Result<Self, String> {
        let mut text = input.trim();
        let mut member = Member::default();
        if text.starts_with(['+', '-', '#', '~']) {
            member.visibility = text[..1].into();
            text = text[1..].trim();
        }
        for _ in 0..2 {
            if let Some(rest) = text.strip_prefix("static ") {
                member.is_static = true;
                text = rest.trim();
            } else if let Some(rest) = text.strip_prefix("abstract ") {
                member.is_abstract = true;
                text = rest.trim();
            }
        }
        if method {
            let open = text
                .find('(')
                .ok_or("메서드는 이름(매개변수): 반환타입 형식으로 입력하세요")?;
            let close = text
                .rfind(')')
                .filter(|i| *i > open)
                .ok_or("메서드 매개변수의 닫는 괄호가 필요합니다")?;
            member.name = text[..open].trim().into();
            member.parameters = text[open + 1..close].trim().into();
            member.type_name = text[close + 1..]
                .trim()
                .strip_prefix(':')
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .ok_or("반환 타입을 : 뒤에 입력하세요")?
                .into();
        } else {
            let (left, value) = text
                .split_once('=')
                .map(|(l, v)| (l, Some(v)))
                .unwrap_or((text, None));
            let (name, ty) = left
                .split_once(':')
                .ok_or("속성은 이름: 타입 = 기본값 형식으로 입력하세요")?;
            member.name = name.trim().into();
            member.type_name = ty.trim().into();
            member.value = value.unwrap_or("").trim().into();
        }
        if member.name.is_empty() || member.type_name.is_empty() {
            return Err("이름과 타입을 입력하세요".into());
        }
        Ok(member)
    }
    pub fn text(&self, method: bool) -> String {
        let mut s = if method {
            format!(
                "{} {}({}): {}",
                self.visibility, self.name, self.parameters, self.type_name
            )
        } else {
            format!("{} {}: {}", self.visibility, self.name, self.type_name)
        };
        if !method && !self.value.is_empty() {
            s.push_str(&format!(" = {}", self.value));
        }
        s
    }
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SizeMode {
    Auto,
    #[default]
    Manual,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UseCaseSpec {
    pub goal: String,
    pub description: String,
    pub primary_actor: String,
    pub preconditions: String,
    pub basic_flow: String,
    pub alternatives: String,
    pub postconditions: String,
    pub sequences: Vec<Id>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub name: String,
    pub attributes: Vec<Member>,
    pub methods: Vec<Member>,
    pub layout: Layout,
    #[serde(default)]
    pub sizing: SizeMode,
    #[serde(default)]
    pub hide_empty: bool,
    #[serde(default)]
    pub specification: UseCaseSpec,
    #[serde(default)]
    pub boundary: Option<Id>,
}
impl Node {
    pub fn new(kind: NodeKind, pos: Point) -> Self {
        let (w, h) = match kind {
            NodeKind::Actor => (90., 130.),
            NodeKind::UseCase => (180., 80.),
            NodeKind::Boundary | NodeKind::Package => (500., 350.),
            _ => (230., 150.),
        };
        Self {
            id: id(),
            kind,
            name: kind.label().into(),
            attributes: vec![],
            methods: vec![],
            layout: Layout::new(pos.x, pos.y, w, h),
            sizing: if matches!(
                kind,
                NodeKind::Class
                    | NodeKind::Interface
                    | NodeKind::AbstractClass
                    | NodeKind::Enumeration
            ) {
                SizeMode::Auto
            } else {
                SizeMode::Manual
            },
            hide_empty: false,
            specification: UseCaseSpec::default(),
            boundary: None,
        }
    }
    pub fn fit(&mut self) {
        self.layout.size = crate::text_layout::class_layout(self, None).size;
    }
    pub fn is_classifier(&self) -> bool {
        matches!(
            self.kind,
            NodeKind::Class | NodeKind::AbstractClass | NodeKind::Interface | NodeKind::Enumeration
        )
    }
    pub fn reflow(&mut self) {
        if self.sizing == SizeMode::Auto && self.is_classifier() {
            self.fit();
        }
        if self.kind == NodeKind::Actor {
            let lines =
                crate::text_layout::wrap(&self.name, 14., (self.layout.size.x - 8.).max(20.));
            self.layout.size.y = self.layout.size.y.max(lines.len() as f32 * 22. + 46.);
        }
        if self.kind == NodeKind::UseCase {
            let lines = crate::text_layout::wrap(&self.name, 15., self.layout.size.x * 0.72);
            self.layout.size.y = self.layout.size.y.max(lines.len() as f32 * 23. + 30.);
        }
        if self.kind == NodeKind::Note {
            let lines = crate::text_layout::wrap(&self.name, 14., self.layout.size.x - 28.);
            self.layout.size.y = self.layout.size.y.max(lines.len() as f32 * 23. + 40.);
        }
    }
}
pub fn text_width(s: &str, size: f32) -> f32 {
    static FACE: std::sync::OnceLock<ttf_parser::Face<'static>> = std::sync::OnceLock::new();
    let face =
        FACE.get_or_init(|| ttf_parser::Face::parse(crate::scene::FONT, 0).expect("bundled font"));
    s.chars()
        .map(|c| {
            face.glyph_index(c)
                .and_then(|g| face.glyph_hor_advance(g))
                .unwrap_or(face.units_per_em()) as f32
        })
        .sum::<f32>()
        / face.units_per_em() as f32
        * size
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RelationKind {
    Association,
    Dependency,
    Generalization,
    Realization,
    Aggregation,
    Composition,
    Include,
    Extend,
    Comment,
}
impl RelationKind {
    pub const ALL: [Self; 9] = [
        Self::Association,
        Self::Dependency,
        Self::Generalization,
        Self::Realization,
        Self::Aggregation,
        Self::Composition,
        Self::Include,
        Self::Extend,
        Self::Comment,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Association => "연관",
            Self::Dependency => "의존",
            Self::Generalization => "일반화 → 상위",
            Self::Realization => "구현 → 인터페이스",
            Self::Aggregation => "집합 (시작=전체)",
            Self::Composition => "합성 (시작=전체)",
            Self::Include => "include → 포함 대상",
            Self::Extend => "extend → 기본",
            Self::Comment => "주석 연결",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Relation {
    pub id: Id,
    pub source: Id,
    pub target: Id,
    pub kind: RelationKind,
    pub name: String,
    pub source_role: String,
    pub target_role: String,
    pub source_multiplicity: String,
    pub target_multiplicity: String,
    pub navigable: bool,
    pub bend: Option<f32>,
    #[serde(default)]
    pub waypoints: Vec<Point>,
}
impl Relation {
    pub fn new(source: Id, target: Id, kind: RelationKind) -> Self {
        Self {
            id: id(),
            source,
            target,
            kind,
            name: String::new(),
            source_role: String::new(),
            target_role: String::new(),
            source_multiplicity: String::new(),
            target_multiplicity: String::new(),
            navigable: false,
            bend: None,
            waypoints: vec![],
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: Id,
    pub name: String,
    pub actor: bool,
    #[serde(default)]
    pub type_ref: Option<Id>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Sync,
    Async,
    Return,
    Create,
    Destroy,
}
impl MessageKind {
    pub const ALL: [Self; 5] = [
        Self::Sync,
        Self::Async,
        Self::Return,
        Self::Create,
        Self::Destroy,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sync => "동기 호출",
            Self::Async => "비동기",
            Self::Return => "반환",
            Self::Create => "객체 생성",
            Self::Destroy => "객체 소멸",
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub id: Id,
    pub source: Id,
    pub target: Id,
    pub kind: MessageKind,
    pub text: String,
    #[serde(default)]
    pub method_ref: Option<Id>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Activation {
    pub id: Id,
    pub participant: Id,
    pub start: Id,
    pub end: Id,
    #[serde(default)]
    pub automatic: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum FragmentKind {
    Alt,
    Opt,
    Loop,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Fragment {
    pub id: Id,
    pub kind: FragmentKind,
    pub condition: String,
    pub else_condition: String,
    pub start: Id,
    pub end: Id,
    pub split: Option<Id>,
    #[serde(default)]
    pub participant_start: Option<Id>,
    #[serde(default)]
    pub participant_end: Option<Id>,
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    pub activations: Vec<Activation>,
    pub fragments: Vec<Fragment>,
    #[serde(default)]
    pub auto_activate: bool,
}
impl Sequence {
    pub fn regenerate_activations(&mut self) {
        let existing: HashMap<_, _> = self
            .activations
            .iter()
            .filter(|a| a.automatic)
            .map(|a| ((a.participant, a.start), a.id))
            .collect();
        self.activations.retain(|a| !a.automatic);
        if !self.auto_activate {
            return;
        }
        let Some(last) = self.messages.last().map(|m| m.id) else {
            return;
        };
        let mut stack: Vec<(Id, Id, usize)> = vec![];
        for m in &self.messages {
            if m.kind == MessageKind::Sync
                && !self
                    .activations
                    .iter()
                    .any(|a| !a.automatic && a.participant == m.target && a.start == m.id)
            {
                let index = self.activations.len();
                self.activations.push(Activation {
                    id: existing.get(&(m.target, m.id)).copied().unwrap_or_else(id),
                    participant: m.target,
                    start: m.id,
                    end: last,
                    automatic: true,
                });
                stack.push((m.source, m.target, index));
            }
            if m.kind == MessageKind::Return
                && let Some(i) = stack
                    .iter()
                    .rposition(|(caller, callee, _)| *caller == m.target && *callee == m.source)
            {
                let (_, _, index) = stack.remove(i);
                self.activations[index].end = m.id;
            }
        }
    }
    pub fn normalize(&mut self) {
        let ps: HashSet<_> = self.participants.iter().map(|p| p.id).collect();
        self.messages
            .retain(|m| ps.contains(&m.source) && ps.contains(&m.target));
        let order: HashMap<_, _> = self
            .messages
            .iter()
            .enumerate()
            .map(|(i, m)| (m.id, i))
            .collect();
        self.activations.retain(|a| {
            ps.contains(&a.participant)
                && order.contains_key(&a.start)
                && order.contains_key(&a.end)
        });
        self.fragments
            .retain(|f| order.contains_key(&f.start) && order.contains_key(&f.end));
        for a in &mut self.activations {
            if order[&a.start] > order[&a.end] {
                std::mem::swap(&mut a.start, &mut a.end);
            }
        }
        for f in &mut self.fragments {
            if f.participant_start.is_some_and(|id| !ps.contains(&id)) {
                f.participant_start = None;
            }
            if f.participant_end.is_some_and(|id| !ps.contains(&id)) {
                f.participant_end = None;
            }
            if order[&f.start] > order[&f.end] {
                std::mem::swap(&mut f.start, &mut f.end);
            }
            if f.split.is_some_and(|s| {
                !order.contains_key(&s) || order[&s] < order[&f.start] || order[&s] > order[&f.end]
            }) {
                f.split = None;
            }
        }
    }
    pub fn remove_message(&mut self, mid: Id) {
        if let Some(i) = self.messages.iter().position(|m| m.id == mid) {
            let replacement = self
                .messages
                .get(i + 1)
                .or_else(|| i.checked_sub(1).and_then(|j| self.messages.get(j)))
                .map(|m| m.id);
            if let Some(r) = replacement {
                for a in &mut self.activations {
                    if a.start == mid {
                        a.start = r;
                    }
                    if a.end == mid {
                        a.end = r;
                    }
                }
                for f in &mut self.fragments {
                    if f.start == mid {
                        f.start = r;
                    }
                    if f.end == mid {
                        f.end = r;
                    }
                    if f.split == Some(mid) {
                        f.split = Some(r);
                    }
                }
            }
            self.messages.remove(i);
            self.normalize();
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct View {
    pub pan: Point,
    pub zoom: f32,
}
impl Default for View {
    fn default() -> Self {
        Self {
            pan: Point::new(40., 40.),
            zoom: 1.,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Diagram {
    pub id: Id,
    pub name: String,
    pub kind: DiagramKind,
    pub nodes: Vec<Node>,
    pub relations: Vec<Relation>,
    pub sequence: Sequence,
    pub view: View,
}
impl Diagram {
    pub fn new(kind: DiagramKind, name: impl Into<String>) -> Self {
        Self {
            id: id(),
            name: name.into(),
            kind,
            nodes: vec![],
            relations: vec![],
            sequence: Sequence::default(),
            view: View::default(),
        }
    }
    pub fn add_relation(&mut self, r: Relation) -> Result<(), String> {
        crate::validation::relation(self, &r)?;
        self.relations.push(r);
        Ok(())
    }
    pub fn update_relation(&mut self, r: Relation) -> Result<(), String> {
        crate::validation::relation(self, &r)?;
        let slot = self
            .relations
            .iter_mut()
            .find(|old| old.id == r.id)
            .ok_or("수정할 관계를 찾을 수 없습니다")?;
        *slot = r;
        Ok(())
    }
    pub fn delete(&mut self, selected: &HashSet<Id>) {
        self.nodes.retain(|n| !selected.contains(&n.id));
        for n in &mut self.nodes {
            if n.boundary.is_some_and(|id| selected.contains(&id)) {
                n.boundary = None;
            }
        }
        self.relations.retain(|r| {
            !selected.contains(&r.id)
                && !selected.contains(&r.source)
                && !selected.contains(&r.target)
        });
        self.sequence
            .participants
            .retain(|p| !selected.contains(&p.id));
        let messages: Vec<_> = self
            .sequence
            .messages
            .iter()
            .filter(|m| selected.contains(&m.id))
            .map(|m| m.id)
            .collect();
        for m in messages {
            self.sequence.remove_message(m);
        }
        self.sequence
            .activations
            .retain(|a| !selected.contains(&a.id));
        self.sequence
            .fragments
            .retain(|a| !selected.contains(&a.id));
        self.sequence.normalize();
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format_version: u32,
    pub id: Id,
    pub name: String,
    pub diagrams: Vec<Diagram>,
}
impl Project {
    pub fn content_eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.name == other.name
            && self.format_version == other.format_version
            && self.diagrams.len() == other.diagrams.len()
            && self.diagrams.iter().zip(&other.diagrams).all(|(a, b)| {
                a.id == b.id
                    && a.name == b.name
                    && a.kind == b.kind
                    && a.nodes == b.nodes
                    && a.relations == b.relations
                    && a.sequence == b.sequence
            })
    }

    pub fn new(
        name: impl Into<String>,
        kind: DiagramKind,
        diagram_name: impl Into<String>,
    ) -> Self {
        Self {
            format_version: 2,
            id: id(),
            name: name.into(),
            diagrams: vec![Diagram::new(kind, diagram_name)],
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        self.validate_structure()?;
        let issues = self.diagnostics();
        if let Some(issue) = issues.first() {
            return Err(issue.message.clone());
        }
        Ok(())
    }
    pub fn diagnostics(&self) -> Vec<crate::validation::Diagnostic> {
        crate::validation::project_diagnostics(self)
    }
    pub fn synchronize_links(&mut self) {
        crate::references::synchronize(self);
    }
    pub fn validate_structure(&self) -> Result<(), String> {
        if self.format_version != 2 {
            return Err(format!(
                "지원하지 않는 프로젝트 버전: {} (지원: 2)",
                self.format_version
            ));
        }
        let mut ids = HashSet::new();
        let mut insert = |i| {
            if ids.insert(i) {
                Ok(())
            } else {
                Err("중복된 고유 ID가 있습니다".to_string())
            }
        };
        insert(self.id)?;
        for d in &self.diagrams {
            insert(d.id)?;
            if !d.view.zoom.is_finite()
                || !(0.1..=5.).contains(&d.view.zoom)
                || !d.view.pan.x.is_finite()
                || !d.view.pan.y.is_finite()
            {
                return Err("잘못된 화면 좌표입니다".into());
            }
            for n in &d.nodes {
                insert(n.id)?;
                let l = n.layout;
                if [l.pos.x, l.pos.y, l.size.x, l.size.y]
                    .iter()
                    .any(|v| !v.is_finite() || v.abs() > 1_000_000.)
                    || l.size.x < 20.
                    || l.size.y < 20.
                {
                    return Err("요소의 위치/크기가 올바르지 않습니다".into());
                }
                for m in n.attributes.iter().chain(&n.methods) {
                    insert(m.id)?;
                }
            }
            for r in &d.relations {
                insert(r.id)?;
                if r.bend.is_some_and(|v| !v.is_finite())
                    || r.waypoints
                        .iter()
                        .any(|p| !p.x.is_finite() || !p.y.is_finite())
                {
                    return Err("연결선의 참조/경로가 올바르지 않습니다".into());
                }
            }
            let s = &d.sequence;
            let ps: HashSet<_> = s.participants.iter().map(|p| p.id).collect();
            let ms: HashMap<_, _> = s
                .messages
                .iter()
                .enumerate()
                .map(|(i, m)| (m.id, i))
                .collect();
            for p in &s.participants {
                insert(p.id)?;
            }
            for m in &s.messages {
                insert(m.id)?;
                if !ps.contains(&m.source) || !ps.contains(&m.target) {
                    return Err("메시지 참여자를 찾을 수 없습니다".into());
                }
            }
            for a in &s.activations {
                insert(a.id)?;
                if !ps.contains(&a.participant)
                    || !ms.contains_key(&a.start)
                    || !ms.contains_key(&a.end)
                    || ms[&a.start] > ms[&a.end]
                {
                    return Err("활성 구간 참조가 올바르지 않습니다".into());
                }
            }
            for f in &s.fragments {
                insert(f.id)?;
                if !ms.contains_key(&f.start)
                    || !ms.contains_key(&f.end)
                    || ms[&f.start] > ms[&f.end]
                    || f.split.is_some_and(|s| {
                        !ms.contains_key(&s) || ms[&s] < ms[&f.start] || ms[&s] > ms[&f.end]
                    })
                {
                    return Err("프래그먼트 범위가 올바르지 않습니다".into());
                }
            }
        }
        Ok(())
    }
}
