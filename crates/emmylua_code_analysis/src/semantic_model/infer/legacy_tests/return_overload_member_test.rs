#[cfg(test)]
mod test {
    use crate::VirtualWorkspace;

    #[test]
    fn test_return_overload_narrow_through_module_member() {
        let mut ws = VirtualWorkspace::new();
        ws.def(
            r#"
            local M = {}

            ---@return_overload true, integer
            ---@return_overload false, string
            function M.two() end

            local ok, value = M.two()
            if ok then
                a = value
            else
                b = value
            end
            "#,
        );

        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("b"), ws.ty("string"));
    }

    #[test]
    fn test_return_overload_narrow_through_method_call() {
        let mut ws = VirtualWorkspace::new();
        ws.def(
            r#"
            ---@class Reader
            local Reader = {}

            ---@return_overload true, string
            ---@return_overload false, integer
            function Reader:read() end

            local r ---@type Reader
            local ok, value = r:read()
            if not ok then
                return
            end
            a = value
            "#,
        );

        assert_eq!(ws.expr_ty("a"), ws.ty("string"));
    }

    #[test]
    fn test_return_overload_narrow_through_member_alias_of_local_function() {
        let mut ws = VirtualWorkspace::new();
        ws.def(
            r#"
            ---@return_overload true, integer
            ---@return_overload false, string
            local function two() end

            local M = {}
            M.two = two

            local ok, value = M.two()
            if ok then
                a = value
            else
                b = value
            end
            "#,
        );

        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("b"), ws.ty("string"));
    }

    #[test]
    fn test_return_overload_narrow_through_required_library_stub() {
        let mut ws = VirtualWorkspace::new();
        ws.def_file(
            "tryer.lua",
            r#"
            ---@meta tryer

            ---@class tryer
            local tryer = {}

            ---@return_overload true, integer
            ---@return_overload false, string
            function tryer.two() end

            ---@generic T
            ---@param v T
            ---@return_overload true, T
            ---@return_overload false, string
            function tryer.try(v) end

            return tryer
            "#,
        );
        ws.def(
            r#"
            local tryer = require("tryer")

            local ok, value = tryer.two()
            if ok then
                a = value
            else
                b = value
            end

            local n ---@type boolean
            local ok2, value2 = tryer.try(n)
            if ok2 then
                c = value2
            else
                d = value2
            end
            "#,
        );

        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
        assert_eq!(ws.expr_ty("b"), ws.ty("string"));
        assert_eq!(ws.expr_ty("c"), ws.ty("boolean"));
        assert_eq!(ws.expr_ty("d"), ws.ty("string"));
    }

    #[test]
    fn test_return_overload_member_assert_narrows_correlated_value() {
        let mut ws = VirtualWorkspace::new_with_init_std_lib();
        ws.def(
            r#"
            local M = {}

            ---@return_overload true, integer
            ---@return_overload false, string
            function M.two() end

            local ok, value = M.two()
            assert(ok)
            a = value
            "#,
        );

        assert_eq!(ws.expr_ty("a"), ws.ty("integer"));
    }
}
