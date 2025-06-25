mod tests {
    use windmill_parser_ts::parse_expr_for_imports;

    #[test]
    fn test_imports() {
        let code = r#"
        import { foo } from "bar";
        import type { foo } from "bar2";
        import { type foo, bar } from "bar3";
                import { bar, type foo } from "bar7";

        import { type foo, type bar } from "bar4";
        import * as foo from "bar5";
        import foo from "bar6";
        "#;
        let imports = parse_expr_for_imports(code, true).unwrap();
        assert_eq!(imports, vec!["bar", "bar3", "bar5", "bar6", "bar7"]);
    }
}
