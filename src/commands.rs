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
                for m in &d.sequence.messages[a..=b] {
                    selected.insert(m.id);
                }
            }
        }
        for a in &d.sequence.activations {
            if selection.contains(&a.id) {
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
                    || (selected.contains(&m.source) && selected.contains(&m.target))
            })
            .cloned()
            .collect();
        let mids: HashSet<_> = messages.iter().map(|m| m.id).collect();
        let sequence = Sequence {
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
                .filter(|n| selection.contains(&n.id))
                .cloned()
                .collect(),
            relations: d
                .relations
                .iter()
                .filter(|r| {
                    selection.contains(&r.id)
                        || (selection.contains(&r.source) && selection.contains(&r.target))
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
                s.fragments.push(f);
            }
            return out;
        }
        for old in &self.nodes {
            let compatible = if d.kind == DiagramKind::Class {
                matches!(
                    old.kind,
                    NodeKind::Class | NodeKind::Interface | NodeKind::AbstractClass
                )
            } else if d.kind == DiagramKind::UseCase {
                matches!(
                    old.kind,
                    NodeKind::Actor | NodeKind::UseCase | NodeKind::Boundary
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
                m.id = id();
            }
            n.layout.pos.x += 30.;
            n.layout.pos.y += 30.;
            d.nodes.push(n);
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
    for boundary in d
        .nodes
        .iter()
        .filter(|n| selection.contains(&n.id) && n.kind == NodeKind::Boundary)
    {
        for child in &d.nodes {
            if child.kind == NodeKind::UseCase && boundary.layout.contains(child.layout.center()) {
                moving.insert(child.id);
            }
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
}
