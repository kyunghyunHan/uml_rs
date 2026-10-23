//! One set of UML rules for creation, retargeting, diagnostics, and saving.
use crate::model::*;
use std::collections::{HashMap, HashSet};
#[derive(Clone, Debug)]
pub struct Diagnostic {
    pub diagram: Id,
    pub element: Id,
    pub message: String,
}
/// Early UX guidance; completed connections still pass through `relation`.
pub fn connection_start(d: &Diagram, source: Id, kind: RelationKind) -> Result<(), String> {
    let node = d
        .nodes
        .iter()
        .find(|n| n.id == source)
        .ok_or("시작 요소가 없습니다. 시작 대상을 다시 선택하세요")?;
    if d.kind == DiagramKind::UseCase {
        let allowed = match kind {
            RelationKind::Include | RelationKind::Extend => node.kind == NodeKind::UseCase,
            RelationKind::Association | RelationKind::Generalization => {
                matches!(node.kind, NodeKind::Actor | NodeKind::UseCase)
            }
            RelationKind::Comment => true,
            _ => false,
        };
        if !allowed {
            return Err(match kind {
                RelationKind::Include | RelationKind::Extend => {
                    "시작 대상으로 유스케이스를 선택하세요."
                }
                RelationKind::Association | RelationKind::Generalization => {
                    "시작 대상으로 액터 또는 유스케이스를 선택하세요."
                }
                _ => "이 관계는 유스케이스 다이어그램에서 사용할 수 없습니다.",
            }
            .into());
        }
    }
    Ok(())
}
pub fn relation(d: &Diagram, candidate: &Relation) -> Result<(), String> {
    let a = d
        .nodes
        .iter()
        .find(|n| n.id == candidate.source)
        .ok_or("시작 요소가 없습니다. 시작 대상을 다시 선택하세요")?;
    let b = d
        .nodes
        .iter()
        .find(|n| n.id == candidate.target)
        .ok_or("도착 요소가 없습니다. 도착 대상을 다시 선택하세요")?;
    let k = candidate.kind;
    if k == RelationKind::Comment {
        return if a.kind == NodeKind::Note || b.kind == NodeKind::Note {
            Ok(())
        } else {
            Err("주석 연결의 한쪽은 노트여야 합니다".into())
        };
    }
    match d.kind {
        DiagramKind::UseCase => match k {
            RelationKind::Association => {
                if !matches!(
                    (a.kind, b.kind),
                    (NodeKind::Actor, NodeKind::UseCase) | (NodeKind::UseCase, NodeKind::Actor)
                ) {
                    return Err("유스케이스 연관은 액터와 유스케이스 사이에 연결하세요".into());
                }
            }
            RelationKind::Include | RelationKind::Extend => {
                if a.kind != NodeKind::UseCase || b.kind != NodeKind::UseCase {
                    return Err(format!(
                        "{}는 유스케이스끼리 연결할 수 있습니다.",
                        if k == RelationKind::Include {
                            "include"
                        } else {
                            "extend"
                        }
                    ));
                }
            }
            RelationKind::Generalization => {
                if a.kind != b.kind || !matches!(a.kind, NodeKind::Actor | NodeKind::UseCase) {
                    return Err("일반화는 액터끼리 또는 유스케이스끼리 연결하세요".into());
                }
            }
            _ => return Err("이 관계는 유스케이스 다이어그램에서 사용할 수 없습니다".into()),
        },
        DiagramKind::Class => {
            if !a.is_classifier() || !b.is_classifier() {
                return Err("클래스 관계의 양 끝은 클래스·인터페이스·열거형이어야 합니다".into());
            }
            if matches!(k, RelationKind::Include | RelationKind::Extend) {
                return Err("include/extend는 유스케이스에서 사용하세요".into());
            }
            if k == RelationKind::Generalization {
                let same_family = (a.kind == NodeKind::Interface && b.kind == NodeKind::Interface)
                    || (matches!(a.kind, NodeKind::Class | NodeKind::AbstractClass)
                        && matches!(b.kind, NodeKind::Class | NodeKind::AbstractClass));
                if !same_family {
                    return Err("클래스는 클래스, 인터페이스는 인터페이스를 일반화합니다. 클래스→인터페이스는 구현을 사용하세요".into());
                }
            }
            if k == RelationKind::Realization
                && (!matches!(a.kind, NodeKind::Class | NodeKind::AbstractClass)
                    || b.kind != NodeKind::Interface)
            {
                return Err("구현은 구현 클래스에서 인터페이스 방향으로 연결하세요".into());
            }
        }
        DiagramKind::Sequence => return Err("시퀀스에서는 메시지를 추가하세요".into()),
    }
    if matches!(k, RelationKind::Generalization | RelationKind::Realization) && a.id == b.id {
        return Err("자기 자신을 상속하거나 구현할 수 없습니다".into());
    }
    if k == RelationKind::Composition && a.id == b.id {
        return Err("합성에서 전체와 부분은 서로 다른 요소여야 합니다".into());
    }
    if k == RelationKind::Generalization {
        let mut visited = HashSet::new();
        let mut stack = vec![candidate.target];
        while let Some(id) = stack.pop() {
            if id == candidate.source {
                return Err(
                    "일반화가 상속 순환을 만듭니다. 도착은 순환하지 않는 상위 요소여야 합니다"
                        .into(),
                );
            }
            if visited.insert(id) {
                stack.extend(
                    d.relations
                        .iter()
                        .filter(|r| {
                            r.id != candidate.id
                                && r.kind == RelationKind::Generalization
                                && r.source == id
                        })
                        .map(|r| r.target),
                );
            }
        }
    }
    Ok(())
}
pub fn project_diagnostics(p: &Project) -> Vec<Diagnostic> {
    let mut out = vec![];
    let nodes: HashMap<_, _> = p
        .diagrams
        .iter()
        .flat_map(|d| d.nodes.iter())
        .map(|n| (n.id, n))
        .collect();
    let diagrams: HashMap<_, _> = p.diagrams.iter().map(|d| (d.id, d)).collect();
    for d in &p.diagrams {
        let mut issue = |element, message: String| {
            out.push(Diagnostic {
                diagram: d.id,
                element,
                message,
            })
        };
        for r in &d.relations {
            if let Err(e) = relation(d, r) {
                issue(r.id, format!("{} · {}: {e}", d.name, r.kind.label()));
            }
        }
        for n in &d.nodes {
            let allowed = match d.kind {
                DiagramKind::Class => {
                    n.is_classifier() || matches!(n.kind, NodeKind::Package | NodeKind::Note)
                }
                DiagramKind::UseCase => matches!(
                    n.kind,
                    NodeKind::Actor | NodeKind::UseCase | NodeKind::Boundary | NodeKind::Note
                ),
                DiagramKind::Sequence => false,
            };
            if !allowed {
                issue(
                    n.id,
                    format!(
                        "{} 요소는 이 다이어그램 종류에 맞지 않습니다",
                        n.kind.label()
                    ),
                );
            }

            if let Some(parent) = n.boundary
                && !d.nodes.iter().any(|b| {
                    b.id == parent && matches!(b.kind, NodeKind::Boundary | NodeKind::Package)
                })
            {
                issue(n.id, "포함 경계/패키지를 찾을 수 없습니다".into());
            }
            for link in &n.specification.sequences {
                if !diagrams
                    .get(link)
                    .is_some_and(|d| d.kind == DiagramKind::Sequence)
                {
                    issue(
                        n.id,
                        "명세의 관련 시퀀스를 찾을 수 없습니다. 연결을 해제하세요".into(),
                    );
                }
            }
        }
        let seq = &d.sequence;
        for participant in &seq.participants {
            if let Some(link) = participant.type_ref
                && !nodes.get(&link).is_some_and(|n| n.is_classifier())
            {
                issue(
                    participant.id,
                    "참여자의 연결 타입이 삭제되었습니다. 연결을 해제하거나 다시 지정하세요".into(),
                );
            }
        }
        let mut alive: HashMap<Id, bool> = seq
            .participants
            .iter()
            .map(|part| {
                (
                    part.id,
                    !seq.messages
                        .iter()
                        .any(|m| m.target == part.id && m.kind == MessageKind::Create),
                )
            })
            .collect();
        let mut created = HashSet::new();
        for m in &seq.messages {
            if !alive.get(&m.source).copied().unwrap_or(false) {
                issue(
                    m.id,
                    "아직 생성되지 않았거나 이미 소멸한 발신자의 메시지입니다".into(),
                );
            }
            if m.kind == MessageKind::Create {
                if !created.insert(m.target) {
                    issue(
                        m.id,
                        "동일한 참여자 ID를 두 번 생성할 수 없습니다. 새 참여자를 추가하세요"
                            .into(),
                    );
                }
                if alive.get(&m.target) == Some(&true) {
                    issue(m.id, "이미 존재하는 참여자를 다시 생성합니다".into());
                }
                if m.source == m.target {
                    issue(m.id, "자기 자신을 생성할 수 없습니다".into());
                }
                alive.insert(m.target, true);
            } else if !alive.get(&m.target).copied().unwrap_or(false) {
                issue(
                    m.id,
                    "생성 전 또는 소멸 이후의 참여자에게 보내는 메시지입니다".into(),
                );
            }
            if m.kind == MessageKind::Destroy {
                alive.insert(m.target, false);
            }
            if let Some(method) = m.method_ref {
                let target = seq
                    .participants
                    .iter()
                    .find(|p| p.id == m.target)
                    .and_then(|p| p.type_ref)
                    .and_then(|id| nodes.get(&id));
                if !target.is_some_and(|n| n.methods.iter().any(|a| a.id == method)) {
                    issue(m.id,"메시지의 연결 메서드가 수신 타입에 없습니다. 메서드 연결을 해제하거나 수정하세요".into());
                }
            }
        }
    }
    out
}
