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

/// Old false means unspecified, never an explicit non-navigable end.
pub fn relation_fields_v3(project: &mut serde_json::Value) {
    if let Some(diagrams) = project.get_mut("diagrams").and_then(|v| v.as_array_mut()) {
        for d in diagrams {
            if let Some(relations) = d.get_mut("relations").and_then(|v| v.as_array_mut()) {
                for r in relations {
                    if let Some(fields) = r.as_object_mut() {
                        let old = fields
                            .remove("navigable")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                        fields
                            .entry("source_navigation")
                            .or_insert(serde_json::json!("Unspecified"));
                        fields
                            .entry("target_navigation")
                            .or_insert(serde_json::json!(if old {
                                "Navigable"
                            } else {
                                "Unspecified"
                            }));
                    }
                }
            }
        }
    }
}
