use anyhow::anyhow;
use serde_json::json;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};
use yaml_rust::{Yaml, YamlLoader};

pub fn parse_ansible_sig(inner_content: &str) -> anyhow::Result<MainArgSignature> {
    let docs = YamlLoader::load_from_str(inner_content)
        .map_err(|e| anyhow!("Failed to parse yaml: {}", e))?;

    if docs.len() < 2 {
        return Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: None,
        });
    }

    let mut args = vec![];
    if let Yaml::Hash(doc) = &docs[0] {
        for (key, value) in doc {
            match key {
                Yaml::String(key) if key == "extra_vars" => {
                    if let Yaml::Hash(v) = &value {
                        for (key, arg) in v {
                            if let Yaml::String(arg_name) = key {
                                args.push(Arg {
                                    name: arg_name.to_string(),
                                    otyp: None,
                                    typ: parse_ansible_typ(&arg),
                                    default: None,
                                    has_default: false,
                                    oidx: None,
                                })
                            }
                        }
                    }
                }
                Yaml::String(key) if key == "inventory" => {
                    if let Yaml::Array(arr) = value {
                        for (i, inv) in arr.iter().enumerate() {
                            if let Yaml::Hash(inv) = inv {

                                let res_type = inv
                                    .get(&Yaml::String("resource_type".to_string()))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("Unknown")
                                    .to_string();

                                let default = inv
                                    .get(&Yaml::String("default".to_string()))
                                    .and_then(|v| v.as_str())
                                    .map(|v| json!(format!("$res:{}", v)));

                                let name = if i == 0 {
                                    "inventory.ini".to_string()
                                } else {
                                    format!("inventory-{}.ini", i)
                                };

                                let name = inv
                                    .get(&Yaml::String("name".to_string()))
                                    .and_then(|v| v.as_str())
                                    .unwrap_or(name.as_str())
                                    .to_string();

                                args.push(Arg {
                                    name,
                                    otyp: None,
                                    typ: Typ::Resource(res_type),
                                    has_default: default.is_some(),
                                    default,
                                    oidx: None,
                                })
                            }
                        }
                    }
                }
                _ => (),
            }
        }
    }
    Ok(MainArgSignature { star_args: false, star_kwargs: false, args, no_main_func: None })
}

fn parse_ansible_typ(arg: &Yaml) -> Typ {
    if let Yaml::Hash(arg) = arg {
        if let Some(Yaml::String(typ)) = arg.get(&Yaml::String("type".to_string())) {
            match typ.as_ref() {
                "boolean" => Typ::Bool,
                "integer" => Typ::Int,
                "number" => Typ::Float,
                "string" => {
                    if let Some(Yaml::String(fmt)) = arg.get(&Yaml::String("format".to_string())) {
                        match fmt.as_ref() {
                            "date-time" | "datetime" => Typ::Datetime,
                            "email" => Typ::Email,
                            _ => Typ::Str(None),
                        }
                    } else {
                        Typ::Str(None)
                    }
                }
                "object" => {
                    if let Some(Yaml::Hash(props)) =
                        arg.get(&Yaml::String("properties".to_string()))
                    {
                        let mut prop_vec = vec![];
                        for (key, value) in props {
                            if let Yaml::String(key) = key {
                                prop_vec.push(ObjectProperty {
                                    key: key.clone(),
                                    typ: Box::new(parse_ansible_typ(value)),
                                })
                            }
                        }
                        Typ::Object(prop_vec)
                    } else {
                        Typ::Object(vec![])
                    }
                }
                "array" => {
                    if let Some(items) = arg.get(&Yaml::String("items".to_string())) {
                        Typ::List(Box::new(parse_ansible_typ(items)))
                    } else {
                        Typ::List(Box::new(Typ::Unknown))
                    }
                }
                "windmill_resource" => {
                    if let Some(Yaml::String(res_name)) =
                        arg.get(&Yaml::String("resource_type".to_string()))
                    {
                        Typ::Resource(res_name.clone())
                    } else {
                        Typ::Resource("".to_string())
                    }
                }
                _ => Typ::Unknown,
            }
        } else {
            Typ::Unknown
        }
    } else {
        Typ::Unknown
    }
}
