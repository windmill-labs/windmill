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
        export * from "bar8";
        export { foo } from "bar9";
        export { type foo } from "bar10";
        export { foo, type bar } from "bar11";
        export { type foo, type bar } from "bar12";
        export { type foo, bar } from "bar13";
        export { bar, type foo } from "bar14";
        export type { foo } from "bar15";
        "#;
        let imports = parse_expr_for_imports(code, true).unwrap();
        assert_eq!(
            imports,
            ["bar", "bar11", "bar13", "bar14", "bar3", "bar5", "bar6", "bar7", "bar8", "bar9"]
        );
    }
}
