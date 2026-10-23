//! Ordered event layout, lifecycle, activation lanes, and fragment gutters.
use crate::{model::*, text_layout::wrap};
use std::collections::HashMap;
pub struct MessageLayout {
    pub id: Id,
    pub points: Vec<Point>,
    pub labels: Vec<(Point, String)>,
    pub kind: MessageKind,
}
pub struct FragmentLayout {
    pub id: Id,
    pub rect: Layout,
    pub title: Vec<String>,
    pub split: Option<(f32, Vec<String>)>,
    pub depth: usize,
}
pub struct SequenceLayout {
    pub participants: HashMap<Id, Layout>,
    pub centers: HashMap<Id, f32>,
    pub events: HashMap<Id, f32>,
    pub activations: HashMap<Id, Layout>,
    pub fragments: Vec<FragmentLayout>,
    pub messages: Vec<MessageLayout>,
    pub lifelines: HashMap<Id, (f32, f32)>,
    pub destroyed: HashMap<Id, Point>,
}
pub fn layout(s: &Sequence) -> SequenceLayout {
    let mut out = SequenceLayout {
        participants: HashMap::new(),
        centers: HashMap::new(),
        events: HashMap::new(),
        activations: HashMap::new(),
        fragments: vec![],
        messages: vec![],
        lifelines: HashMap::new(),
        destroyed: HashMap::new(),
    };
    let order: HashMap<_, _> = s
        .messages
        .iter()
        .enumerate()
        .map(|(i, m)| (m.id, i))
        .collect();
    let porder: HashMap<_, _> = s
        .participants
        .iter()
        .enumerate()
        .map(|(i, p)| (p.id, i))
        .collect();
    let gap = s
        .messages
        .iter()
        .map(|m| text_width(&m.text, 14.) / 3.)
        .fold(115., f32::max)
        .clamp(115., 300.);
    let mut x = 80.;
    for p in &s.participants {
        let width = (text_width(&p.name, 15.) + 36.).clamp(170., 350.);
        out.centers.insert(p.id, x + width / 2.);
        out.participants.insert(
            p.id,
            Layout::new(x, 20., width, if p.actor { 130. } else { 65. }),
        );
        x += width + gap;
    }
    let mut lines = HashMap::new();
    for m in &s.messages {
        let sx = out.centers.get(&m.source).copied().unwrap_or(0.);
        let tx = out.centers.get(&m.target).copied().unwrap_or(sx);
        let width = if m.source == m.target {
            out.participants
                .get(&m.source)
                .map(|p| p.size.x / 2. + gap - 40.)
                .unwrap_or(180.)
        } else {
            (tx - sx).abs() - 45.
        };
        lines.insert(m.id, wrap(&m.text, 14., width.max(85.)));
    }
    struct Range<'a> {
        f: &'a Fragment,
        a: usize,
        b: usize,
        left: usize,
        right: usize,
        depth: usize,
        top: f32,
        bottom: f32,
        split_index: Option<usize>,
        split_y: Option<f32>,
        title: Vec<String>,
        other: Vec<String>,
    }
    let mut ranges = vec![];
    for f in &s.fragments {
        let (Some(&a), Some(&b)) = (order.get(&f.start), order.get(&f.end)) else {
            continue;
        };
        if s.participants.is_empty() {
            continue;
        }
        let i = f
            .participant_start
            .and_then(|id| porder.get(&id).copied())
            .unwrap_or(0);
        let j = f
            .participant_end
            .and_then(|id| porder.get(&id).copied())
            .unwrap_or(s.participants.len() - 1);
        let left = i.min(j);
        let right = i.max(j);
        let width = (out.participants[&s.participants[left].id].size.x / 2. - 20.).max(85.);
        let split_index = if f.kind == FragmentKind::Alt {
            Some(
                f.split
                    .and_then(|id| order.get(&id).copied())
                    .unwrap_or(if b > a {
                        a + (b - a).div_ceil(2)
                    } else {
                        b + 1
                    }),
            )
        } else {
            None
        };
        ranges.push(Range {
            f,
            a,
            b,
            left,
            right,
            depth: 0,
            top: 0.,
            bottom: 0.,
            split_index,
            split_y: None,
            title: wrap(
                &format!(
                    "{} [{}]",
                    match f.kind {
                        FragmentKind::Alt => "alt",
                        FragmentKind::Opt => "opt",
                        FragmentKind::Loop => "loop",
                    },
                    f.condition
                ),
                14.,
                width - 20.,
            ),
            other: wrap(&format!("[{}]", f.else_condition), 13., width - 20.),
        });
    }
    for i in 0..ranges.len() {
        let r = &ranges[i];
        let depth = ranges
            .iter()
            .enumerate()
            .filter(|(j, q)| {
                *j != i
                    && q.a <= r.a
                    && q.b >= r.b
                    && q.left <= r.left
                    && q.right >= r.right
                    && (q.a < r.a || q.b > r.b || q.left < r.left || q.right > r.right || *j < i)
            })
            .count();
        ranges[i].depth = depth;
    }
    let mut y = 180.;
    for (i, m) in s.messages.iter().enumerate() {
        let mut starts: Vec<_> = ranges
            .iter()
            .enumerate()
            .filter(|(_, r)| r.a == i)
            .map(|(j, r)| (j, r.depth))
            .collect();
        starts.sort_by_key(|a| a.1);
        for (j, _) in starts {
            ranges[j].top = y + 8.;
            y += ranges[j].title.len() as f32 * 23. + 30.;
        }
        for r in &mut ranges {
            if r.split_index == Some(i) {
                r.split_y = Some(y + 7.);
                y += r.other.len() as f32 * 22. + 28.;
            }
        }
        y += lines.get(&m.id).map_or(1, Vec::len) as f32 * 23. + 25.;
        out.events.insert(m.id, y);
        if m.source == m.target {
            y += 36.;
        }
        if m.kind == MessageKind::Create {
            y += 75.;
        }
        let mut ends: Vec<_> = ranges
            .iter()
            .enumerate()
            .filter(|(_, r)| r.b == i)
            .map(|(j, r)| (j, r.depth))
            .collect();
        ends.sort_by_key(|a| std::cmp::Reverse(a.1));
        for (j, _) in ends {
            if ranges[j].split_index == Some(i + 1) {
                ranges[j].split_y = Some(y + 15.);
                y += ranges[j].other.len() as f32 * 22. + 30.;
            }
            y += 20.;
            ranges[j].bottom = y;
            y += 8.;
        }
    }
    let bottom = y + 90.;
    for p in &s.participants {
        let created = s
            .messages
            .iter()
            .find(|m| m.target == p.id && m.kind == MessageKind::Create)
            .and_then(|m| out.events.get(&m.id).copied());
        if let Some(cy) = created
            && let Some(r) = out.participants.get_mut(&p.id)
        {
            r.pos.y = cy - r.size.y / 2.;
        }
        let start = out.participants[&p.id].pos.y + out.participants[&p.id].size.y + 8.;
        let end = s
            .messages
            .iter()
            .find(|m| m.target == p.id && m.kind == MessageKind::Destroy)
            .and_then(|m| {
                out.events
                    .get(&m.id)
                    .map(|y| y + if m.source == m.target { 32. } else { 0. })
            })
            .unwrap_or(bottom);
        out.lifelines.insert(p.id, (start, end));
        if end != bottom {
            out.destroyed
                .insert(p.id, Point::new(out.centers[&p.id], end));
        }
    }
    let mut activation_order: Vec<_> = s
        .activations
        .iter()
        .filter_map(|a| Some((a, *order.get(&a.start)?, *order.get(&a.end)?)))
        .collect();
    activation_order.sort_by_key(|(a, start, end)| (*start, std::cmp::Reverse(*end), a.id));
    let mut placed: Vec<(Id, usize, usize, usize)> = vec![];
    let mut lanes = HashMap::new();
    for (a, start, end) in activation_order {
        let mut lane = 0;
        while placed
            .iter()
            .any(|(p, s, e, l)| *p == a.participant && *l == lane && *s <= end && *e >= start)
        {
            lane += 1;
        }
        placed.push((a.participant, start, end, lane));
        lanes.insert(a.id, lane);
        let sy = out.events[&a.start] - 8.;
        let ey = (out.events[&a.end]
            + if s.messages[order[&a.end]].source == s.messages[order[&a.end]].target {
                42.
            } else {
                25.
            })
        .min(
            out.lifelines
                .get(&a.participant)
                .map(|l| l.1)
                .unwrap_or(bottom),
        );
        let Some(&cx) = out.centers.get(&a.participant) else {
            continue;
        };
        out.activations.insert(
            a.id,
            Layout::new(cx - 7. + lane as f32 * 11., sy, 14., (ey - sy).max(12.)),
        );
    }
    for m in &s.messages {
        let (Some(&sx), Some(&tx), Some(&yy)) = (
            out.centers.get(&m.source),
            out.centers.get(&m.target),
            out.events.get(&m.id),
        ) else {
            continue;
        };
        let i = order[&m.id];
        let bar = |participant: Id, source: bool| -> Option<Layout> {
            s.activations
                .iter()
                .filter(|a| {
                    a.participant == participant
                        && order.get(&a.start).is_some_and(|a| *a <= i)
                        && order.get(&a.end).is_some_and(|a| *a >= i)
                        && !(source
                            && m.kind == MessageKind::Sync
                            && m.source == m.target
                            && order.get(&a.start) == Some(&i))
                })
                .max_by_key(|a| lanes.get(&a.id).copied().unwrap_or(0))
                .and_then(|a| out.activations.get(&a.id).copied())
        };
        let self_call = m.source == m.target;
        let right = tx >= sx;
        let source = bar(m.source, true)
            .map(|b| if right { b.pos.x + b.size.x } else { b.pos.x })
            .unwrap_or(sx);
        let target = if m.kind == MessageKind::Create {
            let r = out.participants[&m.target];
            if right { r.pos.x } else { r.pos.x + r.size.x }
        } else {
            (if self_call && m.kind == MessageKind::Return {
                let mut active: Vec<_> = s
                    .activations
                    .iter()
                    .filter(|a| {
                        a.participant == m.target
                            && order.get(&a.start).is_some_and(|a| *a <= i)
                            && order.get(&a.end).is_some_and(|a| *a >= i)
                    })
                    .collect();
                active.sort_by_key(|a| lanes.get(&a.id).copied().unwrap_or(0));
                active
                    .iter()
                    .rev()
                    .nth(1)
                    .and_then(|a| out.activations.get(&a.id).copied())
            } else {
                bar(m.target, false)
            })
            .map(|b| {
                if self_call || !right {
                    b.pos.x + b.size.x
                } else {
                    b.pos.x
                }
            })
            .unwrap_or(tx)
        };
        let points = if self_call {
            vec![
                Point::new(source, yy),
                Point::new(source + 70., yy),
                Point::new(source + 70., yy + 32.),
                Point::new(target, yy + 32.),
            ]
        } else {
            vec![Point::new(source, yy), Point::new(target, yy)]
        };
        let label_x = if self_call {
            source + 18.
        } else {
            source.min(target) + 12.
        };
        let labels = lines[&m.id]
            .iter()
            .enumerate()
            .map(|(j, line)| {
                (
                    Point::new(
                        label_x,
                        yy - 26. - (lines[&m.id].len() - 1 - j) as f32 * 23.,
                    ),
                    line.clone(),
                )
            })
            .collect();
        out.messages.push(MessageLayout {
            id: m.id,
            points,
            labels,
            kind: m.kind,
        });
    }
    let max_depth = ranges.iter().map(|r| r.depth).max().unwrap_or(0);
    for r in ranges {
        let left = out.participants[&s.participants[r.left].id];
        let right = out.participants[&s.participants[r.right].id];
        let pad = 22. + (max_depth - r.depth) as f32 * 17.;
        let mut right_edge = right.pos.x + right.size.x;
        for m in &out.messages {
            let index = order[&m.id];
            let semantic = &s.messages[index];
            if index >= r.a
                && index <= r.b
                && porder
                    .get(&semantic.source)
                    .is_some_and(|i| *i >= r.left && *i <= r.right)
                && porder
                    .get(&semantic.target)
                    .is_some_and(|i| *i >= r.left && *i <= r.right)
            {
                for (pos, text) in &m.labels {
                    right_edge = right_edge.max(pos.x + text_width(text, 14.) + 12.);
                }
                for point in &m.points {
                    right_edge = right_edge.max(point.x + 12.);
                }
            }
        }

        out.fragments.push(FragmentLayout {
            id: r.f.id,
            rect: Layout::new(
                left.pos.x - pad,
                r.top,
                right_edge - left.pos.x + pad * 2.,
                r.bottom - r.top,
            ),
            title: r.title,
            split: r.split_y.map(|y| (y, r.other)),
            depth: r.depth,
        });
    }
    out.fragments.sort_by_key(|f| f.depth);
    out
}
