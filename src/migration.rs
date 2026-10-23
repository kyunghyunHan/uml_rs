use crate::model::*;
pub fn v1_to_v2(p: &mut Project) {
    p.format_version = 2;
    for d in &mut p.diagrams {
        let boundaries: Vec<_> = d
            .nodes
            .iter()
            .filter(|n| n.kind == NodeKind::Boundary)
            .map(|n| (n.id, n.layout))
            .collect();
        for n in &mut d.nodes {
            if n.kind == NodeKind::UseCase && n.boundary.is_none() {
                n.boundary = boundaries
                    .iter()
                    .filter(|(_, r)| r.contains(n.layout.center()))
                    .min_by(|a, b| (a.1.size.x * a.1.size.y).total_cmp(&(b.1.size.x * b.1.size.y)))
                    .map(|a| a.0);
            }
        }
    }
}
