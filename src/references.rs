use crate::model::*;
use std::collections::HashMap;
pub fn synchronize(p: &mut Project) {
    let types: HashMap<_, _> = p
        .diagrams
        .iter()
        .flat_map(|d| d.nodes.iter())
        .filter(|n| n.is_classifier())
        .map(|n| (n.id, n.name.clone()))
        .collect();
    let methods: HashMap<_, _> = p
        .diagrams
        .iter()
        .flat_map(|d| d.nodes.iter())
        .flat_map(|n| n.methods.iter())
        .map(|m| {
            (
                m.id,
                format!("{}({}): {}", m.name, m.parameters, m.type_name),
            )
        })
        .collect();
    for d in &mut p.diagrams {
        for part in &mut d.sequence.participants {
            if let Some(name) = part.type_ref.and_then(|id| types.get(&id)) {
                part.name = name.clone();
            }
        }
        for m in &mut d.sequence.messages {
            if let Some(name) = m.method_ref.and_then(|id| methods.get(&id)) {
                m.text = name.clone();
            }
        }
    }
}
