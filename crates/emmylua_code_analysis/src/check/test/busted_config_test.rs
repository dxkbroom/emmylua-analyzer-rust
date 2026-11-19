//! Built-in busted/luassert addons (`test.framework`) and the busted nudge diagnostics.

#[cfg(test)]
mod tests {
    use lsp_types::NumberOrString;
    use tokio_util::sync::CancellationToken;

    use crate::{
        DiagnosticCode, Emmyrc, EmmyrcTestFramework, VirtualWorkspace, WorkspaceFolder,
        load_resource_from_include_dir,
    };

    const SPEC: &str = r#"
        local match = require("luassert.match")
        local spy = require("luassert.spy")

        describe("thing", function()
            before_each(function() end)

            it("works", function()
                assert.are.same({ 1 }, { 1 })
                assert.is_true(true)
                assert.is_not.equal(1, 2)
                local s = spy.new(function() end)
                s()
                assert.spy(s).was.called(1)
                assert.is_truthy(match.is_string())
            end)
        end)
    "#;

    fn workspace(framework: Option<EmmyrcTestFramework>) -> VirtualWorkspace {
        let mut emmyrc = Emmyrc::default();
        emmyrc.test.framework = framework;
        VirtualWorkspace::new_with_config(emmyrc)
    }

    fn workspace_auto_detected() -> VirtualWorkspace {
        let mut emmyrc = Emmyrc::default();
        emmyrc.test.detected_framework = Some(EmmyrcTestFramework::Busted);
        VirtualWorkspace::new_with_config(emmyrc)
    }

    fn codes(ws: &mut VirtualWorkspace, file_name: &str, source: &str) -> Vec<String> {
        let file_id = ws.def_file(file_name, source);
        ws.analysis
            .diagnose_file(file_id, CancellationToken::new())
            .unwrap_or_default()
            .into_iter()
            .filter_map(|diagnostic| match diagnostic.code {
                Some(NumberOrString::String(code)) => Some(code),
                _ => None,
            })
            .collect()
    }

    fn has(codes: &[String], code: DiagnosticCode) -> bool {
        codes.iter().any(|c| c == code.get_name())
    }

    fn addon_file_count(ws: &VirtualWorkspace) -> usize {
        ws.analysis
            .db
            .file_ids()
            .iter()
            .filter_map(|id| ws.analysis.db.file_path(*id))
            .filter(|path| path.components().any(|c| c.as_os_str() == "addons"))
            .count()
    }

    #[test]
    fn test_busted_framework_resolves_globals_and_modules() {
        let mut ws = workspace(Some(EmmyrcTestFramework::Busted));
        let codes = codes(&mut ws, "spec/thing_spec.lua", SPEC);
        for code in [
            DiagnosticCode::UndefinedGlobal,
            DiagnosticCode::UndefinedField,
            DiagnosticCode::UnresolvedRequire,
            DiagnosticCode::BustedNotConfigured,
            DiagnosticCode::BustedAutoDetected,
        ] {
            assert!(!has(&codes, code), "unexpected {code:?}: {codes:?}");
        }
    }

    #[test]
    fn test_no_framework_describe_is_undefined() {
        let mut ws = workspace(None);
        assert_eq!(addon_file_count(&ws), 0);
        let codes = codes(&mut ws, "spec/thing_spec.lua", SPEC);
        assert!(has(&codes, DiagnosticCode::UndefinedGlobal), "{codes:?}");
        assert!(
            has(&codes, DiagnosticCode::BustedNotConfigured),
            "{codes:?}"
        );
    }

    #[test]
    fn test_luassert_only() {
        let mut ws = workspace(Some(EmmyrcTestFramework::Luassert));
        let codes = codes(
            &mut ws,
            "spec/thing_spec.lua",
            r#"
            local assert = require("luassert")
            local match = require("luassert.match")
            assert.are.same({ 1 }, { 1 })
            assert.is_truthy(match.is_string())
            describe("thing", function() end)
            "#,
        );
        assert!(!has(&codes, DiagnosticCode::UnresolvedRequire), "{codes:?}");
        assert!(!has(&codes, DiagnosticCode::UndefinedField), "{codes:?}");
        // busted globals are not loaded, but the framework is configured: no nudge.
        assert!(has(&codes, DiagnosticCode::UndefinedGlobal), "{codes:?}");
        assert!(
            !has(&codes, DiagnosticCode::BustedNotConfigured),
            "{codes:?}"
        );
    }

    #[test]
    fn test_framework_none_disables_everything() {
        let mut emmyrc = Emmyrc::default();
        emmyrc.test.framework = Some(EmmyrcTestFramework::Disabled);
        emmyrc.test.detected_framework = Some(EmmyrcTestFramework::Busted);
        assert_eq!(emmyrc.test.effective_framework(), None);
        let mut ws = VirtualWorkspace::new_with_config(emmyrc);
        assert_eq!(addon_file_count(&ws), 0);
        let codes = codes(&mut ws, "spec/thing_spec.lua", SPEC);
        assert!(
            !has(&codes, DiagnosticCode::BustedNotConfigured),
            "{codes:?}"
        );
        assert!(
            !has(&codes, DiagnosticCode::BustedAutoDetected),
            "{codes:?}"
        );
    }

    #[test]
    fn test_auto_detected_nudge() {
        let mut ws = workspace_auto_detected();
        let codes = codes(&mut ws, "spec/thing_spec.lua", SPEC);
        assert!(!has(&codes, DiagnosticCode::UndefinedGlobal), "{codes:?}");
        assert_eq!(
            codes
                .iter()
                .filter(|c| *c == DiagnosticCode::BustedAutoDetected.get_name())
                .count(),
            1,
            "{codes:?}"
        );
        assert!(
            !has(&codes, DiagnosticCode::BustedNotConfigured),
            "{codes:?}"
        );
    }

    #[test]
    fn test_nudge_only_in_spec_files() {
        let mut ws = workspace(None);
        let src = "describe('x', function() end)";
        let not_spec = codes(&mut ws, "src/thing.lua", src);
        assert!(!has(&not_spec, DiagnosticCode::BustedNotConfigured));
        let suffix = codes(&mut ws, "test/thing_spec.lua", src);
        assert!(has(&suffix, DiagnosticCode::BustedNotConfigured));
        let dir = codes(&mut ws, "spec/helpers/thing.lua", src);
        assert!(has(&dir, DiagnosticCode::BustedNotConfigured));
    }

    #[test]
    fn test_nudge_skips_local_shadow_and_nested_calls() {
        let mut ws = workspace(None);
        let shadowed = codes(
            &mut ws,
            "spec/a_spec.lua",
            r#"
            local function describe(name, fn) fn() end
            describe("x", function() end)
            "#,
        );
        assert!(!has(&shadowed, DiagnosticCode::BustedNotConfigured));
        let nested = codes(
            &mut ws,
            "spec/b_spec.lua",
            r#"
            local function run()
                it("x", function() end)
            end
            run()
            "#,
        );
        assert!(!has(&nested, DiagnosticCode::BustedNotConfigured));
    }

    #[test]
    fn test_nudge_can_be_disabled() {
        let mut emmyrc = Emmyrc::default();
        emmyrc
            .diagnostics
            .disable
            .push(DiagnosticCode::BustedNotConfigured);
        let mut ws = VirtualWorkspace::new_with_config(emmyrc);
        let codes = codes(&mut ws, "spec/thing_spec.lua", SPEC);
        assert!(
            !has(&codes, DiagnosticCode::BustedNotConfigured),
            "{codes:?}"
        );
    }

    #[test]
    fn test_std_does_not_load_addons() {
        let ws = VirtualWorkspace::new_with_init_std_lib();
        assert_eq!(addon_file_count(&ws), 0);
        // ...but they are bundled.
        assert!(
            load_resource_from_include_dir()
                .iter()
                .any(|file| file.path.ends_with("luassert.lua"))
        );
    }

    /// Mirrors an LSP config reload: clear non-std roots, re-add, reload files.
    #[test]
    fn test_reload_toggles_addons() {
        let mut ws = workspace(Some(EmmyrcTestFramework::Busted));
        let busted_files = addon_file_count(&ws);
        assert!(busted_files > 0);

        let reload = |ws: &mut VirtualWorkspace, framework| {
            let mut emmyrc = Emmyrc::default();
            emmyrc.test.framework = framework;
            ws.analysis.update_config(emmyrc.into());
            ws.analysis.clear_non_std_workspaces();
            ws.analysis
                .add_main_workspace(ws.virtual_url_generator.base.clone());
            let files = ws.analysis.add_test_framework_workspaces();
            ws.analysis.reload_workspace_files(files, Vec::new());
        };

        reload(&mut ws, None);
        assert_eq!(addon_file_count(&ws), 0);
        reload(&mut ws, Some(EmmyrcTestFramework::Luassert));
        let luassert_files = addon_file_count(&ws);
        assert!(luassert_files > 0 && luassert_files < busted_files);
        reload(&mut ws, Some(EmmyrcTestFramework::Busted));
        assert_eq!(addon_file_count(&ws), busted_files);
    }

    #[test]
    fn test_detect_busted_file() {
        let root = std::env::temp_dir().join(format!(
            "emmylua_busted_detect_{}_{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&root).expect("create temp dir");
        let main = WorkspaceFolder::new(root.clone(), false);

        let mut emmyrc = Emmyrc::default();
        emmyrc.detect_test_framework(std::slice::from_ref(&main));
        assert_eq!(emmyrc.test.effective_framework(), None);

        std::fs::write(root.join(".busted"), "return {}").expect("write .busted");
        emmyrc.detect_test_framework(std::slice::from_ref(&main));
        assert_eq!(
            emmyrc.test.effective_framework(),
            Some(EmmyrcTestFramework::Busted)
        );
        assert!(emmyrc.test.is_auto_detected());

        // Library roots do not count.
        let mut lib_only = Emmyrc::default();
        lib_only.detect_test_framework(&[WorkspaceFolder::new(root.clone(), true)]);
        assert_eq!(lib_only.test.effective_framework(), None);

        // Explicit config wins and is not "auto-detected".
        let mut configured = Emmyrc::default();
        configured.test.framework = Some(EmmyrcTestFramework::Luassert);
        configured.detect_test_framework(std::slice::from_ref(&main));
        assert_eq!(
            configured.test.effective_framework(),
            Some(EmmyrcTestFramework::Luassert)
        );
        assert!(!configured.test.is_auto_detected());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn test_config_shape() {
        let emmyrc: Emmyrc =
            serde_json::from_str(r#"{ "test": { "framework": "busted" } }"#).expect("parse");
        assert_eq!(emmyrc.test.framework, Some(EmmyrcTestFramework::Busted));
        let emmyrc: Emmyrc =
            serde_json::from_str(r#"{ "test": { "framework": "none" } }"#).expect("parse");
        assert_eq!(emmyrc.test.framework, Some(EmmyrcTestFramework::Disabled));
        let emmyrc: Emmyrc = serde_json::from_str("{}").expect("parse");
        assert_eq!(emmyrc.test.framework, None);
    }
}
