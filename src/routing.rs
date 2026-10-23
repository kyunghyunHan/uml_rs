use crate::{model::*, scene::segment_distance};
/// Fixed 14-unit actor label; geometric body stretches with the available layout.
pub fn actor_body(r: Layout, name: &str) -> (Layout, Vec<String>) {
    let lines = crate::text_layout::wrap(name, 14., (r.size.x - 8.).max(20.));
    let h = (r.size.y - lines.len() as f32 * 22. - 6.).max(20.);
    (Layout::new(r.pos.x, r.pos.y, r.size.x, h), lines)
}
pub fn actor_arms(r: Layout, name: &str) -> (Point, Point) {
    let (b, _) = actor_body(r, name);
    let c = b.center().x;
    let y = b.pos.y + b.size.y * 0.43;
    (
        Point::new(c - b.size.x * 25. / 90., y),
        Point::new(c + b.size.x * 25. / 90., y),
    )
}
pub fn anchor(n: &Node, toward: Point) -> Point {
    let r = n.layout;
    let c = r.center();
    let dx = toward.x - c.x;
    let dy = toward.y - c.y;
    if n.kind == NodeKind::Actor {
        let (a, b) = actor_arms(r, &n.name);
        if dy < 0. && dx.abs() <= dy.abs() * 0.5 {
            let (body, _) = actor_body(r, &n.name);
            return Point::new(c.x, body.pos.y + body.size.y * 0.03);
        }
        return if dx >= 0. { b } else { a };
    }
    if n.kind == NodeKind::UseCase {
        let t = 1.
            / ((dx / (r.size.x / 2.)).powi(2) + (dy / (r.size.y / 2.)).powi(2))
                .sqrt()
                .max(0.001);
        return Point::new(c.x + dx * t, c.y + dy * t);
    }
    if dx.abs() / r.size.x > dy.abs() / r.size.y {
        Point::new(
            if dx >= 0. {
                r.pos.x + r.size.x
            } else {
                r.pos.x
            },
            c.y,
        )
    } else {
        Point::new(
            c.x,
            if dy >= 0. {
                r.pos.y + r.size.y
            } else {
                r.pos.y
            },
        )
    }
}
fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out = vec![];
    for p in points {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    if out.len() == 1 {
        out.push(Point::new(out[0].x + 1., out[0].y));
    }
    out
}
fn orthogonal(start: Point, end: Point, mid: f32, vertical: bool) -> Vec<Point> {
    if vertical {
        vec![start, Point::new(start.x, mid), Point::new(end.x, mid), end]
    } else {
        vec![start, Point::new(mid, start.y), Point::new(mid, end.y), end]
    }
}
pub fn path(d: &Diagram, r: &Relation) -> Vec<Point> {
    let (Some(a), Some(b)) = (
        d.nodes.iter().find(|n| n.id == r.source),
        d.nodes.iter().find(|n| n.id == r.target),
    ) else {
        return vec![];
    };
    if !r.waypoints.is_empty() {
        let start = anchor(a, r.waypoints[0]);
        let end = anchor(b, *r.waypoints.last().unwrap());
        let mut pts = vec![start];
        for &p in r.waypoints.iter().chain(std::iter::once(&end)) {
            let prev = *pts.last().unwrap();
            if prev.x != p.x && prev.y != p.y {
                pts.push(Point::new(p.x, prev.y));
            }
            pts.push(p);
        }
        return simplify(pts);
    }
    let ac = a.layout.center();
    let bc = b.layout.center();
    if a.id == b.id {
        let start = anchor(a, Point::new(ac.x + 10000., ac.y));
        let end = anchor(a, Point::new(ac.x, ac.y - 10000.));
        return vec![
            start,
            Point::new(start.x + 60., start.y),
            Point::new(start.x + 60., a.layout.pos.y - 45.),
            Point::new(end.x, a.layout.pos.y - 45.),
            end,
        ];
    }
    let horizontal = (ac.x - bc.x).abs() > a.layout.size.x / 2. + b.layout.size.x / 2. + 25.;
    let make = |vertical: bool, mid: f32| {
        let start = anchor(
            a,
            if vertical {
                Point::new(ac.x, mid)
            } else {
                Point::new(mid, ac.y)
            },
        );
        let end = anchor(
            b,
            if vertical {
                Point::new(bc.x, mid)
            } else {
                Point::new(mid, bc.y)
            },
        );
        if vertical && (a.kind == NodeKind::Actor || b.kind == NodeKind::Actor) {
            let outside = |n: &Node, anchor: Point| {
                if n.kind == NodeKind::Actor {
                    if anchor.x >= n.layout.center().x {
                        n.layout.pos.x + n.layout.size.x + 25.
                    } else {
                        n.layout.pos.x - 25.
                    }
                } else {
                    anchor.x
                }
            };
            let ax = outside(a, start);
            let bx = outside(b, end);
            simplify(vec![
                start,
                Point::new(ax, start.y),
                Point::new(ax, mid),
                Point::new(bx, mid),
                Point::new(bx, end.y),
                end,
            ])
        } else {
            simplify(orthogonal(start, end, mid, vertical))
        }
    };
    if let Some(mid) = r.bend {
        return make(!horizontal, mid);
    }
    let mut candidates = vec![make(
        !horizontal,
        if horizontal {
            (ac.x + bc.x) / 2.
        } else {
            (ac.y + bc.y) / 2.
        },
    )];
    for n in &d.nodes {
        if n.id == a.id || n.id == b.id || matches!(n.kind, NodeKind::Boundary | NodeKind::Package)
        {
            continue;
        }
        candidates.push(make(false, n.layout.pos.x - 35.));
        candidates.push(make(false, n.layout.pos.x + n.layout.size.x + 35.));
        candidates.push(make(true, n.layout.pos.y - 35.));
        candidates.push(make(true, n.layout.pos.y + n.layout.size.y + 35.));
    }
    candidates
        .into_iter()
        .min_by(|a, b| score(d, r, a).total_cmp(&score(d, r, b)))
        .unwrap_or_default()
}
fn score(d: &Diagram, r: &Relation, pts: &[Point]) -> f32 {
    let mut score = 0.;
    for w in pts.windows(2) {
        let len = (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
        score += len;
        for n in &d.nodes {
            if n.id == r.source
                || n.id == r.target
                || matches!(n.kind, NodeKind::Boundary | NodeKind::Package)
            {
                continue;
            }
            let b = n.layout;
            let pad = 8.;
            let hit = if (w[0].x - w[1].x).abs() < 0.01 {
                w[0].x > b.pos.x - pad
                    && w[0].x < b.pos.x + b.size.x + pad
                    && w[0].y.min(w[1].y) < b.pos.y + b.size.y + pad
                    && w[0].y.max(w[1].y) > b.pos.y - pad
            } else {
                w[0].y > b.pos.y - pad
                    && w[0].y < b.pos.y + b.size.y + pad
                    && w[0].x.min(w[1].x) < b.pos.x + b.size.x + pad
                    && w[0].x.max(w[1].x) > b.pos.x - pad
            };
            if hit {
                score += 100_000.;
            }
        }
    }
    score
}
pub fn closest_segment(points: &[Point], p: Point) -> Option<usize> {
    points
        .windows(2)
        .enumerate()
        .min_by(|(_, a), (_, b)| {
            segment_distance(p, a[0], a[1]).total_cmp(&segment_distance(p, b[0], b[1]))
        })
        .map(|a| a.0)
}

/// Prefer labels in free space beside the longest segments; avoid covering classifiers.
pub fn label_position(d: &Diagram, points: &[Point], size: Point) -> Point {
    let mut candidates = vec![];
    for w in points.windows(2) {
        let c = Point::new((w[0].x + w[1].x) / 2., (w[0].y + w[1].y) / 2.);
        candidates.extend([
            Point::new(c.x - size.x / 2., c.y - size.y - 8.),
            Point::new(c.x - size.x / 2., c.y + 8.),
            Point::new(c.x + 12., c.y - size.y / 2.),
            Point::new(c.x - size.x - 12., c.y - size.y / 2.),
        ]);
    }
    candidates
        .into_iter()
        .min_by_key(|p| {
            d.nodes
                .iter()
                .filter(|n| !matches!(n.kind, NodeKind::Boundary | NodeKind::Package))
                .filter(|n| {
                    let r = n.layout;
                    p.x < r.pos.x + r.size.x
                        && p.x + size.x > r.pos.x
                        && p.y < r.pos.y + r.size.y
                        && p.y + size.y > r.pos.y
                })
                .count()
        })
        .unwrap_or_default()
}
