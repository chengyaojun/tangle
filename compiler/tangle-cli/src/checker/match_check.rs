use crate::ast::{MatchArm, MatchPattern};
use crate::checker::types::{SumType, Type};

/// Check match exhaustiveness. Returns list of missing variant names.
pub fn check_match_exhaustiveness(sum: &SumType, arms: &[MatchArm]) -> Vec<String> {
    let has_wildcard = arms.iter().any(|a| matches!(a.pattern, MatchPattern::Wildcard));
    if has_wildcard { return vec![]; }

    let mut missing = vec![];
    for variant in &sum.variants {
        if let Type::Primitive(p) = variant {
            let covered = arms.iter().any(|a| match &a.pattern {
                MatchPattern::Variant { name, .. } => name == &p.name,
                _ => false,
            });
            if !covered { missing.push(p.name.clone()); }
        }
    }
    missing
}
