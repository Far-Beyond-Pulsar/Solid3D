use std::collections::{BTreeMap, BTreeSet};

use solid_rs::geometry::SkinWeights;
use solid_rs::{Result, SolidError};

use crate::buffer;
use crate::document::{GltfPrimitive, GltfRoot};

/// Decode every attribute pair before reducing to SolidRS's four-weight layout.
pub(crate) fn read_skin_weights(
    root: &GltfRoot,
    buffers: &[Vec<u8>],
    primitive: &GltfPrimitive,
    vertex_count: usize,
    joint_count: Option<usize>,
) -> Result<(Vec<SkinWeights>, usize)> {
    let mut sets = BTreeSet::new();
    for name in primitive.attributes.keys() {
        if let Some(suffix) = name
            .strip_prefix("JOINTS_")
            .or_else(|| name.strip_prefix("WEIGHTS_"))
        {
            sets.insert(
                suffix
                    .parse::<u32>()
                    .map_err(|_| SolidError::parse(format!("invalid skin attribute {name}")))?,
            );
        }
    }
    if sets.is_empty() {
        return Ok((Vec::new(), 0));
    }
    let mut attributes = Vec::with_capacity(sets.len());
    for set in sets {
        let read = |semantic: &str| -> Result<Vec<f32>> {
            let name = format!("{semantic}_{set}");
            let index = *primitive.attributes.get(&name).ok_or_else(|| {
                SolidError::parse(format!("missing paired skin attribute {name}"))
            })?;
            let accessor = root
                .accessors
                .get(index)
                .ok_or_else(|| SolidError::parse(format!("invalid accessor for {name}")))?;
            let valid_component = match semantic {
                "JOINTS" => matches!(accessor.component_type, 5121 | 5123) && !accessor.normalized,
                _ => {
                    accessor.component_type == 5126
                        || (matches!(accessor.component_type, 5121 | 5123) && accessor.normalized)
                }
            };
            if accessor.type_ != "VEC4" || accessor.count != vertex_count || !valid_component {
                return Err(SolidError::parse(format!("invalid skin accessor {name}")));
            }
            buffer::read_f32(root, buffers, index)
        };
        attributes.push((read("JOINTS")?, read("WEIGHTS")?));
    }
    let mut vertices = Vec::with_capacity(vertex_count);
    let mut reduced = 0;
    let mut influences = BTreeMap::new();
    for vertex in 0..vertex_count {
        influences.clear();
        for (joints, weights) in &attributes {
            for component in 0..4 {
                let offset = vertex * 4 + component;
                let weight = weights[offset];
                if !weight.is_finite() || weight <= 0.0 {
                    continue;
                }
                // UNSIGNED_BYTE/SHORT are exactly representable as f32.
                let joint = joints[offset] as u16;
                if joint_count.is_some_and(|count| usize::from(joint) >= count) {
                    return Err(SolidError::parse(format!(
                        "vertex {vertex} references joint {joint} outside its skin palette"
                    )));
                }
                *influences.entry(joint).or_insert(0.0f64) += f64::from(weight);
            }
        }
        let (weights, discarded) = strongest_four(&influences);
        reduced += usize::from(discarded);
        vertices.push(weights);
    }
    Ok((vertices, reduced))
}

fn strongest_four(influences: &BTreeMap<u16, f64>) -> (SkinWeights, bool) {
    let mut ranked: Vec<_> = influences
        .iter()
        .map(|(&joint, &weight)| (joint, weight))
        .collect();
    ranked.sort_by(|a, b| b.1.total_cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    let discarded = ranked.len() > 4;
    ranked.truncate(4);
    // Accumulate in f64 so valid, large f32 inputs cannot overflow the sum.
    let total: f64 = ranked.iter().map(|(_, weight)| weight).sum();
    let mut out = SkinWeights::default();
    for (slot, (joint, weight)) in ranked.into_iter().enumerate() {
        out.joints[slot] = joint;
        out.weights[slot] = (weight / total) as f32;
    }
    // An empty palette stays zero; the host decides how to bind unweighted vertices.
    (out, discarded)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reduce(pairs: &[(u16, f64)]) -> (SkinWeights, bool) {
        let mut merged = BTreeMap::new();
        for &(joint, weight) in pairs {
            *merged.entry(joint).or_insert(0.0) += weight;
        }
        strongest_four(&merged)
    }

    #[test]
    fn duplicates_are_combined_before_ranking() {
        let (vertex, discarded) =
            reduce(&[(0, 0.1), (1, 0.3), (2, 0.2), (3, 0.2), (4, 0.2), (0, 0.3)]);
        assert!(discarded);
        assert_eq!(vertex.joints, [0, 1, 2, 3]);
        assert!((vertex.weights.iter().sum::<f32>() - 1.0).abs() < 1e-6);
        assert!((vertex.weights[0] - 0.4 / 1.1).abs() < 1e-6);
    }

    #[test]
    fn empty_and_extreme_finite_weights_do_not_divide_by_zero_or_overflow() {
        assert_eq!(reduce(&[]).0, SkinWeights::default());
        let (vertex, discarded) =
            reduce(&[(u16::MAX, f64::from(f32::MAX)), (1, f64::from(f32::MAX))]);
        assert!(!discarded);
        assert_eq!(vertex.joints, [1, u16::MAX, 0, 0]);
        assert_eq!(vertex.weights, [0.5, 0.5, 0.0, 0.0]);
    }
}
