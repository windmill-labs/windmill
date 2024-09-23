use anyhow::anyhow;
use serde_json::json;
use windmill_parser::{Arg, MainArgSignature, ObjectProperty, Typ};
use yaml_rust::{Yaml, YamlEmitter, YamlLoader};

pub fn parse_ansible_sig(inner_content: &str) -> anyhow::Result<MainArgSignature> {
    let docs = YamlLoader::load_from_str(inner_content)
        .map_err(|e| anyhow!("Failed to parse yaml: {}", e))?;

    if docs.len() < 2 {
        return Ok(MainArgSignature {
            star_args: false,
            star_kwargs: false,
            args: vec![],
            no_main_func: None,
            has_preprocessor: None,
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
                    for inv in parse_inventories(value)? {
                        if inv.pinned_resource.is_some() {
                            continue;
                        }
                        args.push(Arg {
                            name: inv.name,
                            otyp: None,
                            typ: Typ::Resource(inv.resource_type.unwrap_or("Unknown".to_string())),
                            has_default: inv.default.is_some(),
                            default: inv.default.map(|v| json!(format!("$res:{}", v))),
                            oidx: None,
                        })
                    }
                }
                _ => (),
            }
        }
    }
    Ok(MainArgSignature {
        star_args: false,
        star_kwargs: false,
        args,
        no_main_func: None,
        has_preprocessor: None,
    })
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

#[derive(Debug)]
pub struct FileResource {
    pub resource_path: String,
    pub target_path: String,
}

#[derive(Debug, Clone)]
pub struct AnsibleInventory {
    pub default: Option<String>,
    pub name: String,
    resource_type: Option<String>,
    pub pinned_resource: Option<String>,
}
#[derive(Debug)]
pub struct AnsibleRequirements {
    pub python_reqs: Vec<String>,
    pub collections: Option<String>,
    pub file_resources: Vec<FileResource>,
    pub inventories: Vec<AnsibleInventory>,
}

fn parse_inventories(inventory_yaml: &Yaml) -> anyhow::Result<Vec<AnsibleInventory>> {
    if let Yaml::Array(arr) = inventory_yaml {
        let mut ret = vec![];
        for (i, inv) in arr.iter().enumerate() {
            if let Yaml::Hash(inv) = inv {
                let resource_type = inv
                    .get(&Yaml::String("resource_type".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                let windmill_path = inv
                    .get(&Yaml::String("default".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

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

                let pin = inv
                    .get(&Yaml::String("resource".to_string()))
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                ret.push(AnsibleInventory {
                    default: windmill_path,
                    name,
                    resource_type,
                    pinned_resource: pin,
                });
            }
        }
        return Ok(ret);
    }
    return Err(anyhow!("Invalid inventory definition"));
}

pub fn parse_ansible_reqs(
    inner_content: &str,
) -> anyhow::Result<(String, Option<AnsibleRequirements>, String)> {
    let mut logs = String::new();
    let docs = YamlLoader::load_from_str(inner_content)
        .map_err(|e| anyhow!("Failed to parse yaml: {}", e))?;

    if docs.len() < 2 {
        return Ok((logs, None, inner_content.to_string()));
    }

    let mut ret = AnsibleRequirements {
        python_reqs: vec![],
        collections: None,
        file_resources: vec![],
        inventories: vec![],
    };

    if let Yaml::Hash(doc) = &docs[0] {
        for (key, value) in doc {
            match key {
                Yaml::String(key) if key == "dependencies" => {
                    if let Yaml::Hash(deps) = value {
                        if let Some(galaxy_requirements) =
                            deps.get(&Yaml::String("galaxy".to_string()))
                        {
                            let mut out_str = String::new();
                            let mut emitter = YamlEmitter::new(&mut out_str);
                            emitter.dump(galaxy_requirements)?;
                            ret.collections = Some(out_str);
                        }
                        if let Some(Yaml::Array(py_reqs)) =
                            deps.get(&Yaml::String("python".to_string()))
                        {
                            ret.python_reqs = py_reqs
                                .iter()
                                .map(|d| d.as_str().map(|s| s.to_string()))
                                .filter_map(|x| x)
                                .collect();
                        }
                    }
                }
                Yaml::String(key) if key == "file_resources" => {
                    if let Yaml::Array(file_resources) = value {
                        let resources: anyhow::Result<Vec<FileResource>> =
                            file_resources.iter().map(parse_file_resource).collect();
                        ret.file_resources.append(&mut resources?);
                    }
                }
                Yaml::String(key) if key == "extra_vars" => {}
                Yaml::String(key) if key == "inventory" => {
                    ret.inventories = parse_inventories(value)?;
                }
                Yaml::String(key) => logs.push_str(&format!("\nUnknown field `{}`. Ignoring", key)),
                _ => (),
            }
        }
    }
    let mut out_str = String::new();
    let mut emitter = YamlEmitter::new(&mut out_str);

    for i in 1..docs.len() {
        emitter.dump(&docs[i])?;
    }
    Ok((logs, Some(ret), out_str))
}

fn parse_file_resource(yaml: &Yaml) -> anyhow::Result<FileResource> {
    if let Yaml::Hash(f) = yaml {
        if let Some(Yaml::String(resource_path)) = f.get(&Yaml::String("resource".to_string())) {
            let target_path = f
                .get(&Yaml::String("target".to_string()))
                .and_then(|x| x.as_str())
                .map(|x| x.to_string())
                .ok_or(anyhow!(
                    "No `target` provided for file resource {}. Please input a target relative path for the ansible playbook to see this file.",
                    resource_path
                ))?;
            return Ok(FileResource { resource_path: resource_path.clone(), target_path });
        }
        return Err(anyhow!(
            "File resource should have a `resource` field, linking to a text file resource"
        ));
    }
    return Err(anyhow!("Invalid file resource: Should be a dictionary."));
}
