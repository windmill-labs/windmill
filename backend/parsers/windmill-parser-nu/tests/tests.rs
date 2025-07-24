#[cfg(test)]
mod test {
    use serde_json::json;
    use windmill_parser::{Arg, MainArgSignature, ObjectType, Typ};
    use windmill_parser_nu::parse_nu_signature;

    #[test]
    fn test_nu_no_main_sig() {
        assert!(parse_nu_signature("").is_err());
    }
    #[test]
    fn test_nu_any_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ a, b , c, d] {}
            "#,
        )
        .unwrap();
        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "a".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "b".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "c".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "d".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_optional_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [foo?] {}
            "#,
        )
        .unwrap();

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "foo".into(),
                    otyp: None,
                    typ: Typ::Unknown,
                    default: Some(serde_json::Value::Null),
                    has_default: true,
                    oidx: None
                },],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_simple_typed_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ foo: string, bar: int] {}
            "#,
        )
        .unwrap();

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "foo".into(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "bar".into(),
                        otyp: None,
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_complete_typed_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [
                    a1: any,
                    a2: bool,
                    a3: int,
                    a4: float,
                    a5: datetime,
                    a6: string,
                    a7: record,
                    a8: list,
                    a9: table,
                    a10: nothing,
                    ] {}
            "#,
        )
        .unwrap();

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "a1".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a2".into(),
                        otyp: None,
                        typ: Typ::Bool,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a3".into(),
                        otyp: None,
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a4".into(),
                        otyp: None,
                        typ: Typ::Float,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a5".into(),
                        otyp: None,
                        typ: Typ::Datetime,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a6".into(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a7".into(),
                        otyp: None,
                        typ: Typ::Object(ObjectType::new(None, Some(vec![]))),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a8".into(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Unknown)),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a9".into(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Object(ObjectType::new(None, Some(vec![]))))),
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "a10".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_default_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ foo = "Foo", bar: string = "Bar", bazz = 3 ] {}
            "#,
        )
        .unwrap();

        println!("{}", serde_json::to_string_pretty(&sig).unwrap());

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "foo".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: Some(json!("Foo")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        name: "bar".into(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: Some(json!("Bar")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        name: "bazz".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: Some(json!(3)),
                        has_default: true,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_preprocessor_sig() {}

    #[test]
    fn test_nu_flags_sig() {
        assert!(parse_nu_signature(
            r#"
                def main [--flag] {}
            "#,
        )
        .is_err());
    }
    #[test]
    fn test_nu_rest_sig() {
        assert!(parse_nu_signature(
            r#"
                def main [...foo: string] {}
            "#,
        )
        .is_err())
    }

    // #[test]
    // fn test_nu_dynamically_sized_sig() {
    //     parse_nu_signature(
    //         r#"
    //             def main [] {

    //             }
    //         "#,
    //     );
    // }
    // #[test]
    // fn test_nu_record_sig() {
    //     let sig = parse_nu_signature(
    //         r#"
    //             def main [ foo: record<a: string, b, c: number, d> ] { }
    //         "#,
    //     )
    //     .unwrap();

    //     assert_eq!(
    //         MainArgSignature {
    //             star_args: false,
    //             star_kwargs: false,
    //             args: vec![Arg {
    //                 name: "foo".into(),
    //                 otyp: None,
    //                 typ: Typ::Object(vec![
    //                     ObjectProperty { key: "a".into(), typ: Box::new(Typ::Str(None)) },
    //                     ObjectProperty { key: "b".into(), typ: Box::new(Typ::Unknown) },
    //                     ObjectProperty { key: "c".into(), typ: Box::new(Typ::Float) },
    //                     ObjectProperty { key: "d".into(), typ: Box::new(Typ::Unknown) },
    //                 ]),
    //                 default: None,
    //                 has_default: false,
    //                 oidx: None
    //             },],
    //             no_main_func: Some(false),
    //             has_preprocessor: None,
    //         },
    //         sig
    //     );
    // }

    #[test]
    fn test_nu_list_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ foo: list<number> ] { }
            "#,
        )
        .unwrap();

        println!("{}", serde_json::to_string_pretty(&sig).unwrap());

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "foo".into(),
                    otyp: None,
                    typ: Typ::List(Box::new(Typ::Float)),
                    default: None,
                    has_default: false,
                    oidx: None
                },],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    #[test]
    fn test_nu_list_full_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ a, foo: list<number> = [ 2, 3, 4 ], b ] { }
            "#,
        )
        .unwrap();

        println!("{}", serde_json::to_string_pretty(&sig).unwrap());

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "a".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "foo".into(),
                        otyp: None,
                        typ: Typ::List(Box::new(Typ::Float)),
                        default: Some(json!([2, 3, 4])),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        name: "b".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }

    #[test]
    fn test_nu_datetime_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [ foo: datetime ] { }
            "#,
        )
        .unwrap();

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![Arg {
                    name: "foo".into(),
                    otyp: None,
                    typ: Typ::Datetime,
                    default: None,
                    has_default: false,
                    oidx: None
                },],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    // TODO: Re-enable for V1
    // #[test]
    // fn test_nu_table_sig() {
    //     let sig = parse_nu_signature(
    //         r#"
    //             def main [ foo: table<a, b: number, c: string>] { }
    //         "#,
    //     )
    //     .unwrap();

    //     assert_eq!(
    //         MainArgSignature {
    //             star_args: false,
    //             star_kwargs: false,
    //             args: vec![Arg {
    //                 name: "foo".into(),
    //                 otyp: None,
    //                 typ: Typ::List(Box::new(Typ::Object(vec![
    //                     ObjectProperty { key: "a".into(), typ: Box::new(Typ::Unknown) },
    //                     ObjectProperty { key: "b".into(), typ: Box::new(Typ::Float) },
    //                     ObjectProperty { key: "c".into(), typ: Box::new(Typ::Str(None)) },
    //                 ]))),
    //                 default: None,
    //                 has_default: false,
    //                 oidx: None
    //             },],
    //             no_main_func: Some(false),
    //             has_preprocessor: None,
    //         },
    //         sig
    //     );
    // }

    #[test]
    fn test_nu_wrapup_sig() {
        let sig = parse_nu_signature(
            r#"
                def main [a  ,b :int,c? , d: string = "foo", bi?: any] {}
            "#,
        )
        .unwrap();

        assert_eq!(
            MainArgSignature {
                star_args: false,
                star_kwargs: false,
                args: vec![
                    Arg {
                        name: "a".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "b".into(),
                        otyp: None,
                        typ: Typ::Int,
                        default: None,
                        has_default: false,
                        oidx: None
                    },
                    Arg {
                        name: "c".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: Some(serde_json::Value::Null),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        name: "d".into(),
                        otyp: None,
                        typ: Typ::Str(None),
                        default: Some(json!("foo")),
                        has_default: true,
                        oidx: None
                    },
                    Arg {
                        name: "bi".into(),
                        otyp: None,
                        typ: Typ::Unknown,
                        default: Some(serde_json::Value::Null),
                        has_default: true,
                        oidx: None
                    }
                ],
                no_main_func: Some(false),
                has_preprocessor: None,
            },
            sig
        );
    }
    // #[test]
    // fn test_nu_wrapup_nested_sig() {
    //     let sig = parse_nu_signature(
    //         r#"
    //             def main [
    //                 baz: string,
    //                 foo: record<a: string,
    //                             b,
    //                             c: list<number>,
    //                             d: record<a1, b1, c1>>
    //                      =
    //                      {
    //                         a: "a",
    //                         b: 3,
    //                         c: [ 2, 3, 4 ],
    //                         d: {
    //                             a: true,
    //                             b: false,
    //                             c: true
    //                         }
    //                     }
    //                     ] { }
    //         "#,
    //     )
    //     .unwrap();

    //     println!("{}", serde_json::to_string_pretty(&sig).unwrap());

    //     assert_eq!(
    //         MainArgSignature {
    //             star_args: false,
    //             star_kwargs: false,
    //             args: vec![
    //                 Arg {
    //                     name: "baz".into(),
    //                     otyp: None,
    //                     typ: Typ::Str(None),
    //                     default: None,
    //                     has_default: false,
    //                     oidx: None
    //                 },
    //                 Arg {
    //                     name: "foo".into(),
    //                     otyp: None,
    //                     typ: Typ::Object(vec![
    //                         ObjectProperty { key: "a".into(), typ: Box::new(Typ::Str(None)) },
    //                         ObjectProperty { key: "b".into(), typ: Box::new(Typ::Unknown) },
    //                         ObjectProperty {
    //                             key: "c".into(),
    //                             typ: Box::new(Typ::List(Box::new(Typ::Float)))
    //                         },
    //                         ObjectProperty {
    //                             key: "d".into(),
    //                             typ: Box::new(Typ::Object(vec![
    //                                 ObjectProperty {
    //                                     key: "a1".into(),
    //                                     typ: Box::new(Typ::Unknown)
    //                                 },
    //                                 ObjectProperty {
    //                                     key: "b1".into(),
    //                                     typ: Box::new(Typ::Unknown)
    //                                 },
    //                                 ObjectProperty {
    //                                     key: "c1".into(),
    //                                     typ: Box::new(Typ::Unknown)
    //                                 }
    //                             ]))
    //                         },
    //                     ]),
    //                     default: Some(json!({
    //                         "a": "a",
    //                         "b": 3,
    //                         "c": [
    //                           2,
    //                           3,
    //                           4
    //                         ],
    //                         "d": {
    //                           "a": true,
    //                           "b": false,
    //                           "c": true
    //                         }
    //                     })),
    //                     has_default: true,
    //                     oidx: None
    //                 },
    //             ],
    //             no_main_func: Some(false),
    //             has_preprocessor: None,
    //         },
    //         sig
    //     );
    // }
    #[test]
    fn test_nu_nested_extra_types() {
        assert_eq!(
            parse_nu_signature(
                r#"
                def main [a: list<int>] {}
            "#,
            )
            .is_err(),
            true
        );
        assert_eq!(
            parse_nu_signature(
                r#"
                def main [a: list<float>] {}
            "#,
            )
            .is_err(),
            true
        );
        assert_eq!(
            parse_nu_signature(
                r#"
                def main [a: list<datetime>] {}
            "#,
            )
            .is_err(),
            true
        );
        assert_eq!(
            parse_nu_signature(
                r#"
                def main [a: list<binary>] {}
            "#,
            )
            .is_err(),
            true
        );
        assert_eq!(
            parse_nu_signature(
                r#"
                def main [a: list<list<int>>] {}
            "#,
            )
            .is_err(),
            true
        );
    }
}
