use crate::model::*;
use std::collections::{HashMap, HashSet};
#[derive(Default)]
pub struct History {
    undo: Vec<Project>,
    redo: Vec<Project>,
}
impl History {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn record(&mut self, before: Project, after: &Project) {
        if !before.content_eq(after) {
            self.undo.push(before);
            if self.undo.len() > 100 {
                self.undo.remove(0);
            }
            self.redo.clear();
        }
    }
    pub fn undo(&mut self, p: &mut Project) {
        if let Some(mut old) = self.undo.pop() {
            preserve_views(p, &mut old);
            self.redo.push(std::mem::replace(p, old));
        }
    }
    pub fn redo(&mut self, p: &mut Project) {
        if let Some(mut new) = self.redo.pop() {
            preserve_views(p, &mut new);
            self.undo.push(std::mem::replace(p, new));
        }
    }
}
fn preserve_views(current: &Project, target: &mut Project) {
    for d in &mut target.diagrams {
        if let Some(old) = current.diagrams.iter().find(|a| a.id == d.id) {
            d.view = old.view;
        }
    }
}
#[derive(Default, Clone)]
pub struct Clipboard {
    nodes: Vec<Node>,
    relations: Vec<Relation>,
    sequence: Sequence,
    source_diagram: Option<Id>,
}
impl Clipboard {
    pub fn copy(d: &Diagram, selection: &HashSet<Id>) -> Self {
        let mut selected = selection.clone();
        loop {
            let children: Vec<_> = d
                .nodes
                .iter()
                .filter(|n| {
                    n.boundary.is_some_and(|id| selected.contains(&id)) && !selected.contains(&n.id)
                })
                .map(|n| n.id)
                .collect();
            if children.is_empty() {
                break;
            }
            selected.extend(children);
        }

        for f in &d.sequence.fragments {
            if selection.contains(&f.id) {
                let a = d
                    .sequence
                    .messages
                    .iter()
                    .position(|m| m.id == f.start)
                    .unwrap_or(0);
                let b = d
                    .sequence
                    .messages
                    .iter()
                    .position(|m| m.id == f.end)
                    .unwrap_or(a);
                if let (Some(left), Some(right)) = (
                    f.participant_start
                        .and_then(|id| d.sequence.participants.iter().position(|p| p.id == id)),
                    f.participant_end
                        .and_then(|id| d.sequence.participants.iter().position(|p| p.id == id)),
                ) {
                    for p in &d.sequence.participants[left.min(right)..=left.max(right)] {
                        selected.insert(p.id);
                    }
                }
                for m in &d.sequence.messages[a..=b] {
                    selected.insert(m.id);
                }
            }
        }
        for a in &d.sequence.activations {
            if selection.contains(&a.id) {
                selected.insert(a.participant);
                selected.insert(a.start);
                selected.insert(a.end);
            }
        }
        for m in &d.sequence.messages {
            if selected.contains(&m.id) {
                selected.insert(m.source);
                selected.insert(m.target);
            }
        }
        let messages: Vec<_> = d
            .sequence
            .messages
            .iter()
            .filter(|m| {
                selected.contains(&m.id)
                    || (selection.contains(&m.source) && selection.contains(&m.target))
            })
            .cloned()
            .collect();
        let mids: HashSet<_> = messages.iter().map(|m| m.id).collect();
        let sequence = Sequence {
            auto_activate: false,
            participants: d
                .sequence
                .participants
                .iter()
                .filter(|p| selected.contains(&p.id))
                .cloned()
                .collect(),
            messages,
            activations: d
                .sequence
                .activations
                .iter()
                .filter(|a| {
                    selected.contains(&a.participant)
                        && mids.contains(&a.start)
                        && mids.contains(&a.end)
                })
                .cloned()
                .collect(),
            fragments: d
                .sequence
                .fragments
                .iter()
                .filter(|f| {
                    mids.contains(&f.start)
                        && mids.contains(&f.end)
                        && f.split.is_none_or(|s| mids.contains(&s))
                })
                .cloned()
                .collect(),
        };
        Self {
            source_diagram: Some(d.id),
            sequence,
            nodes: d
                .nodes
                .iter()
                .filter(|n| selected.contains(&n.id))
                .cloned()
                .collect(),
            relations: d
                .relations
                .iter()
                .filter(|r| {
                    selection.contains(&r.id)
                        || (selected.contains(&r.source) && selected.contains(&r.target))
                })
                .cloned()
                .collect(),
        }
    }
    pub fn paste(&self, d: &mut Diagram) -> HashSet<Id> {
        let mut map = HashMap::new();
        let mut out = HashSet::new();
        if d.kind == DiagramKind::Sequence {
            let s = &mut d.sequence;
            for old in &self.sequence.participants {
                let mut p = old.clone();
                p.id = id();
                map.insert(old.id, p.id);
                out.insert(p.id);
                s.participants.push(p);
            }
            for old in &self.sequence.messages {
                let mut m = old.clone();
                m.id = id();
                map.insert(old.id, m.id);
                m.source = map[&old.source];
                m.target = map[&old.target];
                out.insert(m.id);
                s.messages.push(m);
            }
            for old in &self.sequence.activations {
                let mut a = old.clone();
                a.id = id();
                a.participant = map[&a.participant];
                a.start = map[&a.start];
                a.end = map[&a.end];
                s.activations.push(a);
            }
            for old in &self.sequence.fragments {
                let mut f = old.clone();
                f.id = id();
                f.start = map[&f.start];
                f.end = map[&f.end];
                f.split = f.split.map(|s| map[&s]);
                f.participant_start = f.participant_start.and_then(|p| map.get(&p).copied());
                f.participant_end = f.participant_end.and_then(|p| map.get(&p).copied());
                s.fragments.push(f);
            }
            return out;
        }
        for old in &self.nodes {
            let compatible = if d.kind == DiagramKind::Class {
                matches!(
                    old.kind,
                    NodeKind::Class
                        | NodeKind::Interface
                        | NodeKind::AbstractClass
                        | NodeKind::Note
                        | NodeKind::Enumeration
                        | NodeKind::Package
                )
            } else if d.kind == DiagramKind::UseCase {
                matches!(
                    old.kind,
                    NodeKind::Actor | NodeKind::UseCase | NodeKind::Boundary | NodeKind::Note
                )
            } else {
                false
            };
            if !compatible {
                continue;
            }
            let mut n = old.clone();
            n.id = id();
            map.insert(old.id, n.id);
            out.insert(n.id);
            for m in n.attributes.iter_mut().chain(&mut n.methods) {
                let old = m.id;
                m.id = id();
                map.insert(old, m.id);
            }
            n.layout.pos.x += 30.;
            n.layout.pos.y += 30.;
            d.nodes.push(n);
        }
        for n in &mut d.nodes {
            if out.contains(&n.id) {
                n.boundary = n.boundary.and_then(|old| {
                    map.get(&old)
                        .copied()
                        .or_else(|| (self.source_diagram == Some(d.id)).then_some(old))
                });
            }
        }
        for old in &self.relations {
            let resolve = |id: Id| {
                map.get(&id).copied().or_else(|| {
                    (self.source_diagram == Some(d.id) && d.nodes.iter().any(|n| n.id == id))
                        .then_some(id)
                })
            };
            if let (Some(source), Some(target)) = (resolve(old.source), resolve(old.target)) {
                let mut r = old.clone();
                r.id = id();
                r.source = source;
                r.target = target;
                r.bend = r.bend.map(|x| x + 30.);
                for p in &mut r.waypoints {
                    p.x += 30.;
                    p.y += 30.;
                }
                if self.nodes.is_empty() {
                    out.insert(r.id);
                }
                d.relations.push(r);
            }
        }
        out
    }
}
pub fn move_nodes(d: &mut Diagram, selection: &HashSet<Id>, delta: Point) {
    let mut moving = selection.clone();
    for child in &d.nodes {
        if child
            .boundary
            .is_some_and(|parent| selection.contains(&parent))
        {
            moving.insert(child.id);
        }
    }
    for n in &mut d.nodes {
        if moving.contains(&n.id) {
            n.layout.pos.x += delta.x;
            n.layout.pos.y += delta.y;
        }
    }
}
pub fn align(d: &mut Diagram, selection: &HashSet<Id>, mode: usize) {
    let layouts: Vec<_> = d
        .nodes
        .iter()
        .filter(|n| selection.contains(&n.id))
        .map(|n| n.layout)
        .collect();
    if layouts.len() < 2 {
        return;
    }
    let left = layouts
        .iter()
        .map(|l| l.pos.x)
        .fold(f32::INFINITY, f32::min);
    let right = layouts
        .iter()
        .map(|l| l.pos.x + l.size.x)
        .fold(f32::NEG_INFINITY, f32::max);
    let top = layouts
        .iter()
        .map(|l| l.pos.y)
        .fold(f32::INFINITY, f32::min);
    let bottom = layouts
        .iter()
        .map(|l| l.pos.y + l.size.y)
        .fold(f32::NEG_INFINITY, f32::max);
    let original: HashMap<_, _> = d.nodes.iter().map(|n| (n.id, n.layout)).collect();
    for n in &mut d.nodes {
        if selection.contains(&n.id) {
            let l = &mut n.layout;
            match mode {
                0 => l.pos.x = left,
                1 => l.pos.x = right - l.size.x,
                2 => l.pos.y = top,
                3 => l.pos.y = bottom - l.size.y,
                4 => l.pos.x = (left + right - l.size.x) / 2.,
                _ => l.pos.y = (top + bottom - l.size.y) / 2.,
            }
        }
    }
    let deltas: HashMap<_, _> = d
        .nodes
        .iter()
        .filter(|n| selection.contains(&n.id))
        .map(|n| {
            let old = original[&n.id];
            (
                n.id,
                Point::new(n.layout.pos.x - old.pos.x, n.layout.pos.y - old.pos.y),
            )
        })
        .collect();
    for n in &mut d.nodes {
        if !selection.contains(&n.id)
            && let Some(delta) = n.boundary.and_then(|id| deltas.get(&id))
        {
            n.layout.pos.x += delta.x;
            n.layout.pos.y += delta.y;
        }
    }
}

/// Distribute selected top-level boxes while retaining explicit containment.
pub fn distribute(d: &mut Diagram, selection: &HashSet<Id>, horizontal: bool) {
    let mut nodes: Vec<_> = d
        .nodes
        .iter()
        .filter(|n| selection.contains(&n.id))
        .map(|n| (n.id, n.layout))
        .collect();
    if nodes.len() < 3 {
        return;
    }
    nodes.sort_by(|a, b| {
        if horizontal {
            a.1.pos.x.total_cmp(&b.1.pos.x)
        } else {
            a.1.pos.y.total_cmp(&b.1.pos.y)
        }
    });
    let axis = |r: Layout| if horizontal { r.pos.x } else { r.pos.y };
    let length = |r: Layout| if horizontal { r.size.x } else { r.size.y };
    let first = axis(nodes[0].1);
    let last = nodes.last().unwrap().1;
    let gap = (axis(last) + length(last) - first - nodes.iter().map(|n| length(n.1)).sum::<f32>())
        / (nodes.len() - 1) as f32;
    let mut cursor = first;
    let mut deltas = HashMap::new();
    for (id, old) in nodes {
        let delta = cursor - axis(old);
        deltas.insert(
            id,
            if horizontal {
                Point::new(delta, 0.)
            } else {
                Point::new(0., delta)
            },
        );
        cursor += length(old) + gap;
    }
    for n in &mut d.nodes {
        if let Some(delta) = deltas
            .get(&n.id)
            .or_else(|| n.boundary.and_then(|id| deltas.get(&id)))
        {
            n.layout.pos.x += delta.x;
            n.layout.pos.y += delta.y;
        }
    }
}
pub fn duplicate_diagram(d: &Diagram) -> Diagram {
    let selection = d
        .nodes
        .iter()
        .map(|n| n.id)
        .chain(d.sequence.participants.iter().map(|p| p.id))
        .chain(d.sequence.messages.iter().map(|m| m.id))
        .collect();
    let clip = Clipboard::copy(d, &selection);
    let mut out = Diagram::new(d.kind, format!("{} 복사", d.name));
    clip.paste(&mut out);
    out.sequence.auto_activate = d.sequence.auto_activate;
    out.view = d.view;
    out
}

/// Materialize manual elbows before reversing so their geometry stays reversible.
pub fn reversed_relation(d: &Diagram, relation: &Relation) -> Relation {
    let mut r = relation.clone();
    if !r.waypoints.is_empty() || r.bend.is_some() {
        let points = crate::routing::path(d, &r);
        if points.len() > 2 {
            r.waypoints = points[1..points.len() - 1].to_vec();
        }
        r.bend = None;
    }
    r.swap_ends();
    r
}
