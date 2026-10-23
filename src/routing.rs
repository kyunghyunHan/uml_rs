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
pub fn simplify(points: Vec<Point>) -> Vec<Point> {
    let mut out = vec![];
    for p in points {
        if out.last() != Some(&p) {
            out.push(p);
        }
    }
    let mut i = 1;
    while i + 1 < out.len() {
        let a: Point = out[i - 1];
        let b: Point = out[i];
        let c: Point = out[i + 1];
        let u = Point::new(b.x - a.x, b.y - a.y);
        let v = Point::new(c.x - b.x, c.y - b.y);
        if (u.x * v.y - u.y * v.x).abs() < 0.001 && u.x * v.x + u.y * v.y >= 0. {
            out.remove(i);
        } else {
            i += 1;
        }
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
fn legacy_path(d: &Diagram, r: &Relation) -> Vec<Point> {
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
        let start = port_position(
            a,
            r.source_port,
            if vertical {
                Point::new(ac.x, mid)
            } else {
                Point::new(mid, ac.y)
            },
        );
        let end = port_position(
            b,
            r.target_port,
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

pub fn port_position(node: &Node, port: ConnectionPoint, toward: Point) -> Point {
    let r = node.layout;
    let p = match port.side {
        PortSide::Auto => return anchor(node, toward),
        PortSide::Top => Point::new(r.pos.x + r.size.x * port.offset, r.pos.y),
        PortSide::Bottom => Point::new(r.pos.x + r.size.x * port.offset, r.pos.y + r.size.y),
        PortSide::Left => Point::new(r.pos.x, r.pos.y + r.size.y * port.offset),
        PortSide::Right => Point::new(r.pos.x + r.size.x, r.pos.y + r.size.y * port.offset),
    };
    if matches!(node.kind, NodeKind::UseCase | NodeKind::Actor) {
        anchor(node, p)
    } else {
        p
    }
}
pub fn port_at(node: &Node, p: Point) -> ConnectionPoint {
    let r = node.layout;
    let distances = [
        (PortSide::Top, (p.y - r.pos.y).abs()),
        (PortSide::Bottom, (p.y - r.pos.y - r.size.y).abs()),
        (PortSide::Left, (p.x - r.pos.x).abs()),
        (PortSide::Right, (p.x - r.pos.x - r.size.x).abs()),
    ];
    let side = distances
        .into_iter()
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .unwrap()
        .0;
    let offset = if matches!(side, PortSide::Top | PortSide::Bottom) {
        (p.x - r.pos.x) / r.size.x
    } else {
        (p.y - r.pos.y) / r.size.y
    };
    ConnectionPoint {
        side,
        offset: offset.clamp(0., 1.),
    }
}
fn outward(node: &Node, port: ConnectionPoint, p: Point, distance: f32) -> Point {
    let side = if port.side == PortSide::Auto {
        if node.kind == NodeKind::Actor && (p.x - node.layout.center().x).abs() > 1. {
            if p.x < node.layout.center().x {
                PortSide::Left
            } else {
                PortSide::Right
            }
        } else {
            port_at(node, p).side
        }
    } else {
        port.side
    };
    let (dx, dy) = match side {
        PortSide::Left => (-distance, 0.),
        PortSide::Right => (distance, 0.),
        PortSide::Bottom => (0., distance),
        _ => (0., -distance),
    };
    Point::new(p.x + dx, p.y + dy)
}
/// Strict interior intersection: touching an edge does not count as crossing the node.
pub fn segment_hits(a: Point, b: Point, r: Layout) -> bool {
    let min = Point::new(r.pos.x + 0.05, r.pos.y + 0.05);
    let max = Point::new(r.pos.x + r.size.x - 0.05, r.pos.y + r.size.y - 0.05);
    let mut lo: f32 = 0.;
    let mut hi: f32 = 1.;
    for (start, delta, lower, upper) in [
        (a.x, b.x - a.x, min.x, max.x),
        (a.y, b.y - a.y, min.y, max.y),
    ] {
        if delta.abs() < 0.0001 {
            if start < lower || start > upper {
                return false;
            }
        } else {
            let t1 = (lower - start) / delta;
            let t2 = (upper - start) / delta;
            lo = lo.max(t1.min(t2));
            hi = hi.min(t1.max(t2));
            if lo > hi {
                return false;
            }
        }
    }
    hi > 0.0001 && lo < 0.9999
}
fn obstacles(d: &Diagram) -> impl Iterator<Item = &Node> {
    d.nodes
        .iter()
        .filter(|n| !matches!(n.kind, NodeKind::Package | NodeKind::Boundary))
}
fn route_score(d: &Diagram, r: &Relation, points: &[Point]) -> f32 {
    let mut cost = points.len() as f32 * 10.;
    for (i, w) in points.windows(2).enumerate() {
        cost += (w[1].x - w[0].x).hypot(w[1].y - w[0].y);
        for n in obstacles(d) {
            // The attachment segment must leave its own border without entering it.
            if segment_hits(w[0], w[1], n.layout) {
                cost += 1_000_000.;
            }
        }
        // Give overlapping existing relationships a small separation preference.
        if i > 0 {
            for other in d.relations.iter().filter(|other| other.id != r.id) {
                if other
                    .waypoints
                    .iter()
                    .any(|p| segment_distance(*p, w[0], w[1]) < 4.)
                {
                    cost += 25.;
                }
            }
        }
    }
    cost
}
fn orthogonal_route(
    d: &Diagram,
    r: &Relation,
    a: &Node,
    b: &Node,
    start: Point,
    end: Point,
) -> Vec<Point> {
    let sa = outward(a, r.source_port, start, 32.);
    let tb = outward(b, r.target_port, end, 32.);
    let mut candidates = vec![];
    if (start.x - end.x).abs() < 0.01 || (start.y - end.y).abs() < 0.01 {
        candidates.push(vec![start, end]);
    }
    let mut xs = vec![(sa.x + tb.x) / 2., sa.x, tb.x];
    let mut ys = vec![(sa.y + tb.y) / 2., sa.y, tb.y];
    for n in obstacles(d) {
        xs.extend([n.layout.pos.x - 36., n.layout.pos.x + n.layout.size.x + 36.]);
        ys.extend([n.layout.pos.y - 36., n.layout.pos.y + n.layout.size.y + 36.]);
    }
    for x in xs {
        candidates.push(simplify(vec![
            start,
            sa,
            Point::new(x, sa.y),
            Point::new(x, tb.y),
            tb,
            end,
        ]));
    }
    for y in ys {
        candidates.push(simplify(vec![
            start,
            sa,
            Point::new(sa.x, y),
            Point::new(tb.x, y),
            tb,
            end,
        ]));
    }
    let best = candidates
        .into_iter()
        .min_by(|x, y| route_score(d, r, x).total_cmp(&route_score(d, r, y)))
        .unwrap();
    if route_score(d, r, &best) < 1_000_000. {
        return best;
    }
    visibility_route(d, sa, tb)
        .map(|middle| simplify([vec![start], middle, vec![end]].concat()))
        .unwrap_or(best)
}
/// A sparse orthogonal visibility grid is only needed when simple doglegs are obstructed.
fn visibility_route(d: &Diagram, start: Point, end: Point) -> Option<Vec<Point>> {
    use std::{cmp::Reverse, collections::BinaryHeap};
    let mut xs = vec![start.x, end.x];
    let mut ys = vec![start.y, end.y];
    for n in obstacles(d) {
        xs.extend([n.layout.pos.x - 16., n.layout.pos.x + n.layout.size.x + 16.]);
        ys.extend([n.layout.pos.y - 16., n.layout.pos.y + n.layout.size.y + 16.]);
    }
    xs.sort_by(f32::total_cmp);
    xs.dedup();
    ys.sort_by(f32::total_cmp);
    ys.dedup();
    // Bound interactive work in exceptionally large diagrams; retain the best scored route.
    if xs.len() * ys.len() > 40_000 {
        return None;
    }
    let width = xs.len();
    let count = width * ys.len();
    let at = |i: usize| Point::new(xs[i % width], ys[i / width]);
    let source =
        xs.iter().position(|x| *x == start.x)? + width * ys.iter().position(|y| *y == start.y)?;
    let target =
        xs.iter().position(|x| *x == end.x)? + width * ys.iter().position(|y| *y == end.y)?;
    let mut costs = vec![u64::MAX; count];
    let mut previous = vec![usize::MAX; count];
    let mut heap = BinaryHeap::new();
    costs[source] = 0;
    heap.push(Reverse((0_u64, source)));
    while let Some(Reverse((cost, i))) = heap.pop() {
        if i == target {
            let mut points = vec![at(i)];
            let mut cur = i;
            while cur != source {
                cur = previous[cur];
                points.push(at(cur));
            }
            points.reverse();
            return Some(simplify(points));
        }
        if cost != costs[i] {
            continue;
        }
        let (x, y) = (i % width, i / width);
        for next in [
            if x > 0 { Some(i - 1) } else { None },
            if x + 1 < width { Some(i + 1) } else { None },
            if y > 0 { Some(i - width) } else { None },
            if y + 1 < ys.len() {
                Some(i + width)
            } else {
                None
            },
        ]
        .into_iter()
        .flatten()
        {
            let a = at(i);
            let b = at(next);
            if obstacles(d).any(|n| segment_hits(a, b, n.layout)) {
                continue;
            }
            let added = ((a.x - b.x).abs() + (a.y - b.y).abs())
                .mul_add(100., 1.)
                .ceil() as u64;
            let new = cost.saturating_add(added);
            if new < costs[next] {
                costs[next] = new;
                previous[next] = i;
                heap.push(Reverse((new, next)));
            }
        }
    }
    None
}
pub fn path(d: &Diagram, r: &Relation) -> Vec<Point> {
    let (Some(a), Some(b)) = (
        d.nodes.iter().find(|n| n.id == r.source),
        d.nodes.iter().find(|n| n.id == r.target),
    ) else {
        return vec![];
    };
    if a.id == b.id || (a.layout.center() == b.layout.center() && r.waypoints.is_empty()) {
        let source = if r.source_port.side == PortSide::Auto {
            ConnectionPoint::centered(PortSide::Right)
        } else {
            r.source_port
        };
        let target = if r.target_port.side == PortSide::Auto {
            ConnectionPoint::centered(PortSide::Top)
        } else {
            r.target_port
        };
        if r.waypoints.is_empty() {
            let s = port_position(a, source, b.layout.center());
            let t = port_position(b, target, a.layout.center());
            let sa = outward(a, source, s, 64.);
            let tb = outward(b, target, t, 32.);
            let middle = if source.side == target.side {
                let corner = Point::new(
                    a.layout.pos.x + a.layout.size.x + 100.,
                    a.layout.pos.y - 80.,
                );
                match (
                    visibility_route(d, sa, corner),
                    visibility_route(d, corner, tb),
                ) {
                    (Some(first), Some(second)) => Some([first, second].concat()),
                    _ => None,
                }
            } else {
                visibility_route(d, sa, tb)
            };
            return middle
                .map(|middle| simplify([vec![s], middle, vec![t]].concat()))
                .unwrap_or_else(|| vec![s, sa, Point::new(sa.x, tb.y), tb, t]);
        }
    }
    if !r.waypoints.is_empty() {
        let start = port_position(a, r.source_port, r.waypoints[0]);
        let end = port_position(b, r.target_port, *r.waypoints.last().unwrap());
        let mut controls = vec![start];
        if r.source_port.side != PortSide::Auto {
            controls.push(outward(a, r.source_port, start, 32.));
        }
        controls.extend(r.waypoints.iter().copied());
        if r.target_port.side != PortSide::Auto {
            controls.push(outward(b, r.target_port, end, 32.));
        }
        controls.push(end);
        let mut pts = vec![controls[0]];
        for &p in &controls[1..] {
            let prev = *pts.last().unwrap();
            if prev.x != p.x && prev.y != p.y {
                pts.push(Point::new(p.x, prev.y));
            }
            pts.push(p);
        }
        return simplify(pts);
    }
    if r.bend.is_some() {
        return legacy_path(d, r);
    }
    let mut start = port_position(a, r.source_port, b.layout.center());
    let mut end = port_position(b, r.target_port, a.layout.center());
    if d.kind == DiagramKind::Class
        && r.source_port.side == PortSide::Auto
        && r.target_port.side == PortSide::Auto
    {
        let ac = a.layout.center();
        let bc = b.layout.center();
        let top = a.layout.pos.y.max(b.layout.pos.y);
        let bottom = (a.layout.pos.y + a.layout.size.y).min(b.layout.pos.y + b.layout.size.y);
        let left = a.layout.pos.x.max(b.layout.pos.x);
        let right = (a.layout.pos.x + a.layout.size.x).min(b.layout.pos.x + b.layout.size.x);
        if bottom - top >= 20. && (ac.x - bc.x).abs() > (a.layout.size.x + b.layout.size.x) / 2. {
            let y = ((ac.y + bc.y) / 2.).clamp(top + 10., bottom - 10.);
            start.y = y;
            end.y = y;
        } else if right - left >= 20.
            && (ac.y - bc.y).abs() > (a.layout.size.y + b.layout.size.y) / 2.
        {
            let x = ((ac.x + bc.x) / 2.).clamp(left + 10., right - 10.);
            start.x = x;
            end.x = x;
        }
    }
    if r.route_mode == RouteMode::Auto && d.kind == DiagramKind::Class {
        let parallel: Vec<_> = d
            .relations
            .iter()
            .filter(|other| {
                (other.source == r.source && other.target == r.target)
                    || (other.source == r.target && other.target == r.source)
            })
            .collect();
        if parallel.len() > 1
            && let Some(index) = parallel.iter().position(|other| other.id == r.id)
        {
            let shift = (index as f32 - (parallel.len() - 1) as f32 / 2.) * 18.;
            for (node, port, p) in [(a, r.source_port, &mut start), (b, r.target_port, &mut end)] {
                if port.side == PortSide::Auto {
                    let side = port_at(node, *p).side;
                    if matches!(side, PortSide::Left | PortSide::Right) {
                        p.y = (p.y + shift).clamp(
                            node.layout.pos.y + 8.,
                            node.layout.pos.y + node.layout.size.y - 8.,
                        );
                    } else {
                        p.x = (p.x + shift).clamp(
                            node.layout.pos.x + 8.,
                            node.layout.pos.x + node.layout.size.x - 8.,
                        );
                    }
                }
            }
        }
    }
    if (start.x - end.x).hypot(start.y - end.y) < 0.01 {
        return vec![
            start,
            Point::new(start.x + 64., start.y),
            Point::new(start.x + 64., start.y - 64.),
            Point::new(end.x, end.y - 64.),
            end,
        ];
    }
    if r.route_mode == RouteMode::Straight {
        return simplify(vec![start, end]);
    }
    if r.route_mode == RouteMode::Auto
        && d.kind == DiagramKind::UseCase
        && r.source_port.side == PortSide::Auto
        && r.target_port.side == PortSide::Auto
    {
        return legacy_path(d, r);
    }
    if r.route_mode == RouteMode::Auto && !obstacles(d).any(|n| segment_hits(start, end, n.layout))
    {
        return simplify(vec![start, end]);
    }
    orthogonal_route(d, r, a, b, start, end)
}

/// Snap a dragged boundary attachment along its edge; node geometry is never changed.
pub fn snap_port(
    d: &Diagram,
    relation: &Relation,
    source: bool,
    pointer: Point,
    tolerance: f32,
) -> (ConnectionPoint, Vec<(Point, Point)>) {
    let id = if source {
        relation.source
    } else {
        relation.target
    };
    let Some(node) = d.nodes.iter().find(|n| n.id == id) else {
        return (ConnectionPoint::default(), vec![]);
    };
    let mut port = port_at(node, pointer);
    let mut candidates = vec![node.layout.center()];
    for other in &d.relations {
        let points = path(d, other);
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            if other.id != relation.id || !source {
                candidates.push(*first);
            }
            if other.id != relation.id || source {
                candidates.push(*last);
            }
        }
    }
    let p = port_position(node, port, pointer);
    let horizontal = matches!(port.side, PortSide::Top | PortSide::Bottom);
    let nearest = candidates
        .into_iter()
        .filter(|c| {
            if horizontal {
                (c.x - p.x).abs() < tolerance
            } else {
                (c.y - p.y).abs() < tolerance
            }
        })
        .min_by(|a, b| {
            let distance = |c: &Point| {
                if horizontal {
                    (c.x - p.x).abs()
                } else {
                    (c.y - p.y).abs()
                }
            };
            distance(a).total_cmp(&distance(b))
        });
    let mut guides = vec![];
    if let Some(c) = nearest {
        port.offset = if horizontal {
            ((c.x - node.layout.pos.x) / node.layout.size.x).clamp(0., 1.)
        } else {
            ((c.y - node.layout.pos.y) / node.layout.size.y).clamp(0., 1.)
        };
        let p = port_position(node, port, pointer);
        guides.push((p, c));
    }
    (port, guides)
}

fn overlap(a: Layout, b: Layout) -> f32 {
    ((a.pos.x + a.size.x).min(b.pos.x + b.size.x) - (a.pos.x.max(b.pos.x))).max(0.)
        * ((a.pos.y + a.size.y).min(b.pos.y + b.size.y) - (a.pos.y.max(b.pos.y))).max(0.)
}
pub fn label_position_avoiding(
    d: &Diagram,
    points: &[Point],
    size: Point,
    occupied: &[Layout],
    segments: &[(Point, Point)],
) -> Point {
    let mut candidates = vec![];
    for w in points.windows(2) {
        for fraction in [0.5, 0.3, 0.7] {
            let c = Point::new(
                w[0].x + (w[1].x - w[0].x) * fraction,
                w[0].y + (w[1].y - w[0].y) * fraction,
            );
            for offset in [10., 28., 48., 72.] {
                candidates.extend([
                    Point::new(c.x - size.x / 2., c.y - size.y - offset),
                    Point::new(c.x - size.x / 2., c.y + offset),
                    Point::new(c.x + offset, c.y - size.y / 2.),
                    Point::new(c.x - size.x - offset, c.y - size.y / 2.),
                ]);
            }
        }
    }
    place_label(d, &candidates, size, occupied, segments)
}
fn place_label(
    d: &Diagram,
    candidates: &[Point],
    size: Point,
    occupied: &[Layout],
    segments: &[(Point, Point)],
) -> Point {
    candidates
        .iter()
        .enumerate()
        .min_by(|(ia, a), (ib, b)| {
            let score = |p: &&Point, i: usize| {
                let box_ = Layout::new(p.x - 3., p.y - 3., size.x + 6., size.y + 6.);
                obstacles(d)
                    .map(|n| overlap(box_, n.layout))
                    .chain(occupied.iter().map(|r| overlap(box_, *r)))
                    .sum::<f32>()
                    * 10000.
                    + segments
                        .iter()
                        .filter(|(a, b)| segment_hits(*a, *b, box_))
                        .count() as f32
                        * 10000.
                    + i as f32
            };
            score(a, *ia).total_cmp(&score(b, *ib))
        })
        .map(|(_, p)| *p)
        .unwrap_or_default()
}
pub fn endpoint_label_position(
    d: &Diagram,
    end: Point,
    toward: Point,
    size: Point,
    occupied: &[Layout],
    segments: &[(Point, Point)],
) -> Point {
    let len = (toward.x - end.x).hypot(toward.y - end.y).max(0.01);
    let u = Point::new((toward.x - end.x) / len, (toward.y - end.y) / len);
    let mut candidates = vec![];
    for distance in [42., 62., 88., 120.] {
        for side in [1., -1.] {
            for gap in [18., 34., 52.] {
                let c = Point::new(
                    end.x + u.x * distance - u.y * gap * side,
                    end.y + u.y * distance + u.x * gap * side,
                );
                candidates.push(Point::new(c.x - size.x / 2., c.y - size.y / 2.));
            }
        }
    }
    place_label(d, &candidates, size, occupied, segments)
}

pub fn path_obstructions(d: &Diagram, r: &Relation) -> Vec<Id> {
    let points = path(d, r);
    obstacles(d)
        .filter(|n| {
            points
                .windows(2)
                .any(|w| segment_hits(w[0], w[1], n.layout))
        })
        .map(|n| n.id)
        .collect()
}
