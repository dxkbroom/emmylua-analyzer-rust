#[cfg(test)]
mod test {
    use crate::VirtualWorkspace;

    const TRY: &str = r#"
        ---@generic T...
        ---@param f fun(): T...
        ---@return_overload true, T...
        ---@return_overload false, string
        local function try(f) end
    "#;

    fn def_with_try(ws: &mut VirtualWorkspace, body: &str) {
        ws.def(&format!("{TRY}\n{body}"));
    }

    #[test]
    fn test_generic_variadic_rows_narrow_both_branches_typed_callback() {
        let mut ws = VirtualWorkspace::new();
        def_with_try(
            &mut ws,
            r#"
            local cb ---@type fun(): integer, boolean
            local ok, a, b = try(cb)
            if ok then
                t_a = a
                t_b = b
            else
                f_a = a
                f_b = b
            end
            "#,
        );

        assert_eq!(ws.expr_ty("t_a"), ws.ty("integer"));
        // The slot after the first `T...` value comes from the spread tail.
        assert_eq!(ws.expr_ty("t_b"), ws.ty("boolean"));
        assert_eq!(ws.expr_ty("f_a"), ws.ty("string"));
        assert_eq!(ws.expr_ty("f_b"), ws.ty("nil"));
    }

    #[test]
    fn test_generic_variadic_rows_narrow_both_branches_literal_callback() {
        let mut ws = VirtualWorkspace::new();
        def_with_try(
            &mut ws,
            r#"
            local ok, a, b = try(function() return 1, true end)
            if ok then
                t_a = a
                t_b = b
            else
                f_a = a
            end
            "#,
        );

        assert_eq!(ws.expr_ty("t_a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("t_b"), ws.ty("boolean"));
        assert_eq!(ws.expr_ty("f_a"), ws.ty("string"));
    }

    #[test]
    fn test_generic_variadic_rows_empty_tail_false_branch_is_not_nil() {
        let mut ws = VirtualWorkspace::new();
        def_with_try(
            &mut ws,
            r#"
            local ok, err = try(function() end)
            if not ok then
                lit_err = err
            end

            local cb ---@type fun()
            local ok2, err2 = try(cb)
            if not ok2 then
                typed_err = err2
            end
            "#,
        );

        assert_eq!(ws.expr_ty("lit_err"), ws.ty("string"));
        assert_eq!(ws.expr_ty("typed_err"), ws.ty("string"));
    }

    #[test]
    fn test_generic_single_tpl_literal_rows_match_declared_type() {
        let mut ws = VirtualWorkspace::new();
        ws.def(
            r#"
            ---@generic T
            ---@param f fun(): T
            ---@return_overload true, T
            ---@return_overload false, string
            local function try1(f) end

            local ok, a = try1(function() return 1 end)
            if ok then
                t_a = a
            else
                f_a = a
            end
            "#,
        );

        // A bare `T` callback return keeps its literal, in the declared type and
        // in the narrowed row alike; nothing is left over in either branch.
        let t_a = ws.expr_ty("t_a");
        assert_eq!(ws.humanize_type(t_a), "1");
        assert_eq!(ws.expr_ty("f_a"), ws.ty("string"));
    }

    #[test]
    fn test_generic_variadic_rows_early_return() {
        let mut ws = VirtualWorkspace::new();
        def_with_try(
            &mut ws,
            r#"
            local cb ---@type fun(): integer, string
            local ok, a, b = try(cb)
            if not ok then
                return
            end
            t_a = a
            t_b = b
            "#,
        );

        assert_eq!(ws.expr_ty("t_a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("t_b"), ws.ty("string"));
    }

    const ASYNC_STUB: &str = r#"
        ---@meta async

        ---@class async.Error
        ---@field kind string

        ---@class async.Scope

        ---@class async.Deferred<T...>

        ---@class async
        local async = {}

        ---@generic T...
        ---@param body fun(scope: async.Scope): T...
        ---@return async.Deferred<T...>
        function async.async(body) end

        ---@generic T...
        ---@param deferred async.Deferred<T...>
        ---@return_overload true, T...
        ---@return_overload false, async.Error
        function async.try_await(deferred) end

        ---@generic T...
        ---@param body fun(scope: async.Scope): T...
        ---@return_overload true, T...
        ---@return_overload false, async.Error
        function async.try_scope(body) end

        return async
    "#;

    #[test]
    fn test_library_stub_try_await_narrows_every_slot() {
        let mut ws = VirtualWorkspace::new();
        ws.def_file("async.lua", ASYNC_STUB);
        ws.def(
            r#"
            local async = require("async")
            local d = async.async(function() return "x", "y" end)
            local ok, x, y = async.try_await(d)
            if ok then
                t_x = x
                t_y = y
            else
                f_x = x
                f_y = y
            end
            "#,
        );

        assert_eq!(ws.expr_ty("t_x"), ws.ty("string"));
        assert_eq!(ws.expr_ty("t_y"), ws.ty("string"));
        assert_eq!(ws.expr_ty("f_x"), ws.ty("async.Error"));
        assert_eq!(ws.expr_ty("f_y"), ws.ty("nil"));
    }

    #[test]
    fn test_library_stub_try_scope_narrows_every_slot() {
        let mut ws = VirtualWorkspace::new();
        ws.def_file("async.lua", ASYNC_STUB);
        ws.def(
            r#"
            local async = require("async")
            local ok, a, b = async.try_scope(function() return 1, 2 end)
            if ok then
                t_a = a
                t_b = b
            else
                f_a = a
            end

            local ok2, err = async.try_scope(function() end)
            if not ok2 then
                empty_err = err
            end
            "#,
        );

        assert_eq!(ws.expr_ty("t_a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("t_b"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("f_a"), ws.ty("async.Error"));
        assert_eq!(ws.expr_ty("empty_err"), ws.ty("async.Error"));
    }
}
