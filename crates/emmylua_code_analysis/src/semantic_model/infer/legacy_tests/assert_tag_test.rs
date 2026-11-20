#[cfg(test)]
mod test {
    use crate::VirtualWorkspace;

    const CHECKS: &str = r#"
        local check = {}

        ---@assert value
        function check.truthy(value) end

        ---@assert value -?
        function check.not_nil(value) end

        ---@assert value true
        function check.is_true(value) end

        ---@assert value false
        function check.is_false(value) end

        ---@assert value nil
        function check.is_nil(value) end

        ---@assert value string
        function check.is_string(value) end

        ---@return integer|false|nil
        local function maybe() end
    "#;

    fn def(ws: &mut VirtualWorkspace, body: &str) {
        ws.def(&format!("{CHECKS}\n{body}"));
    }

    #[test]
    fn test_assert_tag_bare_narrows_to_truthy() {
        let mut ws = VirtualWorkspace::new();
        def(&mut ws, "local x = maybe()\ncheck.truthy(x)\na = x");
        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
    }

    #[test]
    fn test_assert_tag_not_nil_keeps_false() {
        let mut ws = VirtualWorkspace::new();
        def(&mut ws, "local x = maybe()\ncheck.not_nil(x)\na = x");
        assert_eq!(ws.expr_ty("a"), ws.ty("integer|false"));
    }

    #[test]
    fn test_assert_tag_type_replaces() {
        let mut ws = VirtualWorkspace::new();
        def(
            &mut ws,
            r#"
            local x = maybe()
            check.is_nil(x)
            a = x

            local y ---@type any
            check.is_string(y)
            b = y
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("nil"));
        assert_eq!(ws.expr_ty("b"), ws.ty("string"));
    }

    #[test]
    fn test_assert_tag_is_statement_only() {
        let mut ws = VirtualWorkspace::new();
        def(
            &mut ws,
            r#"
            local x = maybe()
            local _ = check.truthy(x)
            a = x
            if check.truthy(x) then
                b = x
            end
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("integer|false|nil"));
        assert_eq!(ws.expr_ty("b"), ws.ty("integer|false|nil"));
    }

    #[test]
    fn test_assert_tag_only_after_the_call() {
        let mut ws = VirtualWorkspace::new();
        def(
            &mut ws,
            r#"
            local x = maybe()
            before = x
            check.truthy(x)
            after = x
            "#,
        );
        assert_eq!(ws.expr_ty("before"), ws.ty("integer|false|nil"));
        assert_eq!(ws.expr_ty("after"), ws.ty("integer"));
    }

    #[test]
    fn test_assert_tag_narrows_through_branch_merge() {
        let mut ws = VirtualWorkspace::new();
        def(
            &mut ws,
            r#"
            local x = maybe()
            local cond ---@type boolean
            if cond then
                print(1)
            end
            check.not_nil(x)
            a = x
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("integer|false"));
    }

    #[test]
    fn test_assert_tag_on_flag_narrows_correlated_return_overload() {
        let mut ws = VirtualWorkspace::new();
        def(
            &mut ws,
            r#"
            ---@return_overload true, integer
            ---@return_overload false, string
            local function two() end

            local ok, v = two()
            check.is_true(ok)
            a = v

            local ok2, v2 = two()
            check.truthy(ok2)
            b = v2

            local ok3, v3 = two()
            check.is_false(ok3)
            c = v3

            local ok4, v4 = two()
            check.not_nil(ok4)
            d = v4
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("b"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("c"), ws.ty("string"));
        // `false` is not nil: nothing is known about the row.
        assert_eq!(ws.expr_ty("d"), ws.ty("integer|string"));
    }

    #[test]
    fn test_assert_tag_on_local_function_and_method() {
        let mut ws = VirtualWorkspace::new();
        ws.def(
            r#"
            ---@assert v -?
            local function ensure(v) end

            ---@class Checker
            local Checker = {}

            ---@assert v -?
            function Checker:ensure(v) end

            ---@type string?
            local s
            ensure(s)
            a = s

            local c ---@type Checker
            ---@type string?
            local t
            c:ensure(t)
            b = t
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("string"));
        assert_eq!(ws.expr_ty("b"), ws.ty("string"));
    }

    #[test]
    fn test_assert_tag_from_required_module() {
        let mut ws = VirtualWorkspace::new();
        ws.def_file(
            "guard.lua",
            r#"
            ---@meta guard
            ---@class guard
            local guard = {}

            ---@assert value -?
            function guard.present(value) end

            return guard
            "#,
        );
        ws.def(
            r#"
            local guard = require("guard")
            ---@type integer?
            local n
            guard.present(n)
            a = n
            "#,
        );
        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
    }
}
