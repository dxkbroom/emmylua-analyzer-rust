//! `---@return_overload` rows, built once for both call-return inference
//! (`vm.rs`) and correlated flow narrowing (`flow.rs`), so the declared type of
//! each result and the rows a guard narrows it with always agree.

use crate::semantic_db::def::SignatureDoc;
use crate::semantic_model::SemanticModel;
use crate::semantic_model::doc_types::DocTypeScope;
use crate::{LuaType, VariadicType};

use super::unify;

/// One `---@return_overload` row after generic substitution, with every
/// variadic spread: `true, T...` with `T... = (integer, string)` becomes the
/// fixed slots `true, integer, string`; an unbounded `R...` / `integer...`
/// becomes the `tail` that fills every later slot.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReturnOverloadRow {
    pub fixed: Vec<LuaType>,
    pub tail: Option<LuaType>,
}

impl ReturnOverloadRow {
    pub fn from_types(types: &[LuaType]) -> Self {
        let mut row = ReturnOverloadRow {
            fixed: Vec::new(),
            tail: None,
        };
        row.spread(types);
        row
    }

    fn spread(&mut self, types: &[LuaType]) {
        for ty in types {
            match ty {
                LuaType::Variadic(variadic) => match variadic.as_ref() {
                    VariadicType::Base(base) | VariadicType::Repeat(base) => {
                        if self.tail.is_none() {
                            self.tail = Some(base.clone());
                        }
                    }
                    VariadicType::Multi(inner) => self.spread(inner),
                },
                _ => self.fixed.push(ty.clone()),
            }
        }
    }

    /// The type of result `index`: a fixed slot, else the tail, else `nil`.
    pub fn slot(&self, index: usize) -> LuaType {
        self.fixed
            .get(index)
            .or(self.tail.as_ref())
            .cloned()
            .unwrap_or(LuaType::Nil)
    }
}

/// Project the `---@return_overload` rows of `docs` in `scope`, substituting
/// `bindings` when the call bound the signature's generics.
pub(crate) fn return_overload_rows(
    model: &SemanticModel,
    docs: &SignatureDoc,
    scope: &DocTypeScope,
    bindings: Option<&unify::TplBindings>,
) -> Vec<ReturnOverloadRow> {
    let mut rows = Vec::with_capacity(docs.return_overload_rows.len());
    let mut index = 0;
    for &len in &docs.return_overload_rows {
        let end = (index + len).min(docs.return_overloads.len());
        let raw: Vec<LuaType> = docs.return_overloads[index..end]
            .iter()
            .map(|(_, syntax)| {
                let ty = model.project_doc_type_in_scope(*syntax, scope);
                match bindings {
                    Some(bindings) => unify::substitute(&ty, bindings),
                    None => ty,
                }
            })
            .collect();
        rows.push(ReturnOverloadRow::from_types(&raw));
        index = end;
    }
    rows
}
