use crate::checker::types::*;
use crate::checker::errors::ErrorRegistry;
use crate::model::TangleDiagnostic;

/// Strip error variants from a sum type when ? operator is used.
/// Returns the ok type(s) remaining after removing error variants.
pub fn check_propagation(ty: &Type, registry: &ErrorRegistry) -> (Type, Vec<TangleDiagnostic>) {
    let mut diags = vec![];
    match ty {
        Type::Sum(sum) => {
            let ok_variants: Vec<Type> = sum.variants.iter()
                .filter(|v| {
                    if let Type::Primitive(p) = v {
                        !registry.is_error(&p.name)
                    } else { true }
                })
                .cloned()
                .collect();

            if ok_variants.is_empty() {
                diags.push(TangleDiagnostic {
                    code: "TANGLE_TYPE_ALL_ERROR".into(),
                    message: "All variants are errors — propagation leaves no ok type".into(),
                    span: crate::model::SourceSpan {
                        file: String::new(), start_line: 0, start_column: 0, end_line: 0, end_column: 0,
                    },
                });
                (Type::Primitive(PrimitiveType { name: "Bool".into() }), diags)
            } else if ok_variants.len() == 1 {
                (ok_variants[0].clone(), diags)
            } else {
                (Type::Sum(SumType { variants: ok_variants }), diags)
            }
        }
        _ => (ty.clone(), diags),
    }
}
