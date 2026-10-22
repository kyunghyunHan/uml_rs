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
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id: Id,
    pub kind: NodeKind,
    pub name: String,
    pub attributes: Vec<Member>,
    pub methods: Vec<Member>,
    pub layout: Layout,
}
impl Node {
    pub fn new(kind: NodeKind, pos: Point) -> Self {
        let (w, h) = match kind {
            NodeKind::Actor => (90., 130.),
            NodeKind::UseCase => (180., 80.),
            NodeKind::Boundary => (500., 350.),
            _ => (230., 150.),
        };
        Self {
            id: id(),
            kind,
            name: kind.label().into(),
            attributes: vec![],
            methods: vec![],
            layout: Layout::new(pos.x, pos.y, w, h),
        }
    }
    pub fn fit(&mut self) {
        let lines: Vec<String> = std::iter::once(self.name.clone())
            .chain(self.attributes.iter().map(|m| m.text(false)))
            .chain(self.methods.iter().map(|m| m.text(true)))
            .collect();
        self.layout.size = Point::new(
            lines
                .iter()
                .map(|s| text_width(s, 14.) + 30.)
                .fold(170., f32::max),
            48. + if self.kind == NodeKind::Interface {
                18.
            } else {
                0.
            } + 24. * (self.attributes.len().max(1) + self.methods.len().max(1)) as f32,
        );
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
}
impl RelationKind {
    pub const ALL: [Self; 8] = [
        Self::Association,
        Self::Dependency,
        Self::Generalization,
        Self::Realization,
        Self::Aggregation,
        Self::Composition,
        Self::Include,
        Self::Extend,
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
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Participant {
    pub id: Id,
    pub name: String,
    pub actor: bool,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageKind {
    Sync,
    Async,
    Return,
}
impl MessageKind {
    pub const ALL: [Self; 3] = [Self::Sync, Self::Async, Self::Return];
    pub fn label(self) -> &'static str {
        match self {
            Self::Sync => "동기 호출",
            Self::Async => "비동기",
            Self::Return => "반환",
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
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Activation {
    pub id: Id,
    pub participant: Id,
    pub start: Id,
    pub end: Id,
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
}
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Sequence {
    pub participants: Vec<Participant>,
    pub messages: Vec<Message>,
    pub activations: Vec<Activation>,
    pub fragments: Vec<Fragment>,
}
impl Sequence {
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
    pub fn delete(&mut self, selected: &HashSet<Id>) {
        self.nodes.retain(|n| !selected.contains(&n.id));
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
            format_version: 1,
            id: id(),
            name: name.into(),
            diagrams: vec![Diagram::new(kind, diagram_name)],
        }
    }
    pub fn validate(&self) -> Result<(), String> {
        if self.format_version != 1 {
            return Err(format!(
                "지원하지 않는 프로젝트 버전: {} (지원: 1)",
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
            let ns: HashSet<_> = d.nodes.iter().map(|n| n.id).collect();
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
                if !ns.contains(&r.source)
                    || !ns.contains(&r.target)
                    || r.bend.is_some_and(|v| !v.is_finite())
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
