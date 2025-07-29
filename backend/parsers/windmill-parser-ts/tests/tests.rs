#[cfg(test)]
mod tests {
    use serde_json::json;
    use windmill_parser::{Arg, MainArgSignature, ObjectProperty, ObjectType, Typ};
    use windmill_parser_ts::{parse_deno_signature, parse_expr_for_imports};

    #[test]
    fn test_imports_basic() {
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

    #[test]
    fn test_parse_empty_main_signature() {
        let code = r#"
        export async function main() {
            return "Hello World";
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_basic_types() {
        let code = r#"
        export async function main(
            str_param: string,
            num_param: number,
            bool_param: boolean,
            any_param: any
        ) {
            return { str_param, num_param, bool_param, any_param };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "str_param".to_string(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "num_param".to_string(),
                        otyp: None,
                        typ: Typ::Float,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "bool_param".to_string(),
                        otyp: None,
                        typ: Typ::Bool,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "any_param".to_string(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_with_defaults() {
        let code = r#"
        export async function main(
            name: string = "World",
            count: number = 42,
            enabled: boolean = true
        ) {
            return { name, count, enabled };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "name".to_string(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: Some(json!("World")),
                        has_default: true,
                        oidx: None,
                    },
                    Arg {
                        name: "count".to_string(),
                        otyp: None,
                        typ: Typ::Float,
                        default: Some(json!(42)),
                        has_default: true,
                        oidx: None,
                    },
                    Arg {
                        name: "enabled".to_string(),
                        otyp: None,
                        typ: Typ::Bool,
                        default: Some(json!(true)),
                        has_default: true,
                        oidx: None,
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_array_types() {
        let code = r#"
        export async function main(
            strings: string[],
            numbers: number[],
            items: any[]
        ) {
            return { strings, numbers, items };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "strings".to_string(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Str(None))),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "numbers".to_string(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Float)),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "items".to_string(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Unknown)),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_resource_types() {
        let code = r#"
        type Postgresql = Resource;

        export async function main(
            database: Postgresql
        ) {
            return { database };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "database".to_string(),
                    otyp: None,
                    typ: Typ::Resource("postgresql".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None,
                },],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_array_resource_types() {
        let code = r#"
        type Postgresql = Foo;

        export async function main(
            database: Postgresql[]
        ) {
            return { database };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "database".to_string(),
                    otyp: None,
                    typ: Typ::List(Box::new(Typ::Resource("postgresql".to_string()))),
                    default: None,
                    has_default: false,
                    oidx: None,
                },],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_interface_object() {
        let code = r#"
        interface UserConfig {
            name: string;
            age: number;
            active?: boolean;
        }
        
        export async function main(config: UserConfig) {
            return config;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "config".to_string(),
                    otyp: None,
                    typ: Typ::Object(ObjectType::new(
                        Some("user_config".to_string()),
                        Some(vec![
                            ObjectProperty::new("name".to_string(), Box::new(Typ::Str(None))),
                            ObjectProperty::new("age".to_string(), Box::new(Typ::Float)),
                            ObjectProperty::new("active".to_string(), Box::new(Typ::Bool)),
                        ])
                    )),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_array_of_objects() {
        let code = r#"
        interface Item {
            id: string;
            value: number;
        }
        
        export async function main(items: Item[]) {
            return items;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "items".to_string(),
                    otyp: None,
                    typ: Typ::List(Box::new(Typ::Object(ObjectType::new(
                        Some("item".to_string()),
                        Some(vec![
                            ObjectProperty::new("id".to_string(), Box::new(Typ::Str(None))),
                            ObjectProperty::new("value".to_string(), Box::new(Typ::Float)),
                        ])
                    )))),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_nested_objects() {
        let code = r#"
        interface Address {
            street: string;
            city: string;
        }
        
        interface Person {
            name: string;
            address: Address;
        }
        
        export async function main(person: Person) {
            return person;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "person".to_string(),
                    otyp: None,
                    typ: Typ::Object(ObjectType::new(
                        Some("person".to_string()),
                        Some(vec![
                            ObjectProperty::new("name".to_string(), Box::new(Typ::Str(None))),
                            ObjectProperty::new(
                                "address".to_string(),
                                Box::new(Typ::Object(ObjectType::new(
                                    Some("address".to_string()),
                                    Some(vec![
                                        ObjectProperty::new(
                                            "street".to_string(),
                                            Box::new(Typ::Str(None))
                                        ),
                                        ObjectProperty::new(
                                            "city".to_string(),
                                            Box::new(Typ::Str(None))
                                        ),
                                    ])
                                )))
                            ),
                        ])
                    )),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_builtin_types() {
        let code = r#"
        export async function main(
            date_param: Date,
            base64_param: Base64,
            email_param: Email,
            sql_param: Sql
        ) {
            return { date_param, base64_param, email_param, sql_param };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "date_param".to_string(),
                        otyp: None,
                        typ: Typ::Datetime,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "base64_param".to_string(),
                        otyp: None,
                        typ: Typ::Bytes,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "email_param".to_string(),
                        otyp: None,
                        typ: Typ::Email,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "sql_param".to_string(),
                        otyp: None,
                        typ: Typ::Sql,
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_optional_parameters() {
        let code = r#"
        export async function main(
            required: string,
            optional?: number,
            with_default: boolean = false
        ) {
            return { required, optional, with_default };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "required".to_string(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None,
                    },
                    Arg {
                        name: "optional".to_string(),
                        otyp: None,
                        typ: Typ::Float,
                        default: None,
                        has_default: true,
                        oidx: None,
                    },
                    Arg {
                        name: "with_default".to_string(),
                        otyp: None,
                        typ: Typ::Bool,
                        default: Some(json!(false)),
                        has_default: true,
                        oidx: None,
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_complex_interface() {
        let code = r#"
        interface Config {
            timeout: number;
            retries: number;
        }
        
        export async function main(
            config: Config,
            tags: string[]
        ) {
            return { config, tags };
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();

        assert_eq!(sig.args.len(), 2);
        assert_eq!(sig.args[0].name, "config");
        assert_eq!(sig.args[0].has_default, false);
        assert!(matches!(sig.args[0].typ, Typ::Object(_)));

        assert_eq!(sig.args[1].name, "tags");
        assert_eq!(sig.args[1].has_default, false);
        assert!(matches!(sig.args[1].typ, Typ::List(_)));
    }

    #[test]
    fn test_parse_no_main_function() {
        let code = r#"
        function helper() {
            return "helper";
        }
        
        const value = 42;
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![],
                no_main_func: Some(true),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_custom_entrypoint() {
        let code = r#"
        export async function customMain(param: string) {
            return param;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, Some("customMain".to_string())).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "param".to_string(),
                    otyp: None,
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_dyn_select() {
        let code = r#"
        export async function main(
            selector: DynSelect_my_options
        ) {
            return selector;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "selector".to_string(),
                    otyp: None,
                    typ: Typ::DynSelect("my_options".to_string()),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(false),
            }
        );
    }

    #[test]
    fn test_parse_invalid_typescript() {
        let code = r#"
        this is not valid typescript code {
        "#;
        let result = parse_deno_signature(code, false, false, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_with_preprocessor() {
        let code = r#"
        export async function preprocessor() {
            return "preprocessed";
        }
        
        export async function main(param: string) {
            return param;
        }
        "#;
        let sig = parse_deno_signature(code, false, false, None).unwrap();
        assert_eq!(
            sig,
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "param".to_string(),
                    otyp: None,
                    typ: Typ::Str(None),
                    default: None,
                    has_default: false,
                    oidx: None,
                }],
                no_main_func: Some(false),
                has_preprocessor: Some(true),
            }
        );
    }
}
