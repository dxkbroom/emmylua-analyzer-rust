//! # busted_config - nudges for the built-in busted support (`test.framework` in .emmyrc.json)
//!
//! Only runs when `test.framework` is not configured, and only on busted spec files
//! (`*_spec.lua` or files under a `spec/` directory) that call a busted block function
//! (`describe`, `it`, ...) at the top level:
//! - `.busted` found, busted auto-enabled -> `BustedAutoDetected`: suggest making it explicit;
//! - nothing configured or detected -> `BustedNotConfigured`: suggest enabling it.
//!
//! One diagnostic per file, on the first top-level busted call.

use std::path::Path;

use emmylua_parser::{LuaAstNode, LuaExpr, LuaStat};

use crate::DiagnosticCode;
use crate::semantic_db::def::DeclKind;
use crate::semantic_model::SemanticModel;

use super::{CheckContext, Checker};

/// Busted functions that are typically called at the top level of a spec file.
const BUSTED_TOP_LEVEL_FUNCTIONS: &[&str] = &[
    "describe",
    "context",
    "insulate",
    "expose",
    "it",
    "spec",
    "test",
    "pending",
    "setup",
    "teardown",
    "lazy_setup",
    "lazy_teardown",
    "strict_setup",
    "strict_teardown",
    "before_each",
    "after_each",
];

pub struct BustedConfigChecker;

impl Checker for BustedConfigChecker {
    const CODES: &[DiagnosticCode] = &[
        DiagnosticCode::BustedNotConfigured,
        DiagnosticCode::BustedAutoDetected,
    ];

    fn check(context: &mut CheckContext<'_>, semantic_model: &SemanticModel<'_>) {
        let Some(emmyrc) = semantic_model.db().config() else {
            return;
        };
        let test = &emmyrc.test;
        if test.framework.is_some() {
            return;
        }
        let code = if test.is_auto_detected() {
            DiagnosticCode::BustedAutoDetected
        } else {
            DiagnosticCode::BustedNotConfigured
        };
        let Some(path) = semantic_model.file_path_of(semantic_model.file_id()) else {
            return;
        };
        if !is_spec_file(&path) {
            return;
        }
        let Some(tree) = semantic_model.syntax_tree() else {
            return;
        };
        let Some(block) = tree.get_chunk_node().get_block() else {
            return;
        };

        for stat in block.get_stats() {
            let LuaStat::CallExprStat(call_stat) = stat else {
                continue;
            };
            let Some(LuaExpr::NameExpr(name_expr)) = call_stat
                .get_call_expr()
                .and_then(|call| call.get_prefix_expr())
            else {
                continue;
            };
            let Some(name) = name_expr.get_name_text() else {
                continue;
            };
            if !BUSTED_TOP_LEVEL_FUNCTIONS.contains(&name.as_str()) {
                continue;
            }
            // A local `describe`/`it` shadows busted; not a busted spec call.
            if let Some(decl_id) = semantic_model.resolve_name(name_expr.get_position())
                && let Some(facts) = semantic_model.file_facts()
                && let Some(decl) = facts.decl_by_id(&decl_id)
                && matches!(decl.kind, DeclKind::Local { .. } | DeclKind::Param)
            {
                continue;
            }

            let message = match code {
                DiagnosticCode::BustedAutoDetected => t!(
                    "Built-in busted support was enabled automatically because a .busted file was found. Set `test.framework` to `busted` in .emmyrc.json to make this explicit."
                )
                .to_string(),
                _ => t!(
                    "This looks like a busted spec ('%{name}' is a busted function), but busted support is not enabled. Set `test.framework` to `busted` in .emmyrc.json, or add a .busted file to the project root, to load the built-in busted and luassert definitions.",
                    name = name
                )
                .to_string(),
            };
            context.add_diagnostic(code, name_expr.get_range(), message);
            return;
        }
    }
}

/// `*_spec.lua` files or files inside a `spec` directory (busted's defaults).
fn is_spec_file(path: &Path) -> bool {
    let is_spec_name = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.ends_with("_spec"));
    is_spec_name
        || path
            .parent()
            .is_some_and(|parent| parent.components().any(|c| c.as_os_str() == "spec"))
}
