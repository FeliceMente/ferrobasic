//! HIR → MIR lowering.

use crate::Program;

/// Lowers a type-checked module, one `ferro_hir::check` returned without
/// diagnostics, to a program that passes [`crate::validate`].
pub fn lower(module: &ferro_hir::Module) -> Program {
    let _ = module;
    todo!("HIR -> MIR lowering (milestone 4c)")
}
