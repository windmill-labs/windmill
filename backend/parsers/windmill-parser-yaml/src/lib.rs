use std::collections::HashMap;

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
                                // if not then it is a static resource or var, so skip
                                // it will be fetched by the worker
                                if let ArgTyp::Typ(typ) = parse_ansible_arg_typ(arg) {
                                    let default = get_default_for_typ(&arg);
                                    args.push(Arg {
                                        name: arg_name.to_string(),
                                        otyp: None,
                                        typ,
                                        has_default: default.is_some(),
                                        default,
                                        oidx: None,
                                    })
                                }
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

fn get_default_for_typ(arg: &Yaml) -> Option<serde_json::Value> {
    if let Yaml::Hash(arg) = arg {
        if let Some(def) = arg.get(&Yaml::String("default".to_string())) {
            if let Some(Yaml::String(typ)) = arg.get(&Yaml::String("type".to_string())) {
                if let Yaml::String(path) = def {
                    if typ.as_str() == "windmill_resource" {
                        return Some(serde_json::Value::String(format!("$res:{}", path)));
                    }
                }
            }
            return Some(yaml_to_json(def));
        }
    }
    None
}

enum ArgTyp {
    Typ(Typ),
    StaticVar(String),
    StaticResource(String),
}

fn parse_ansible_arg_typ(arg: &Yaml) -> ArgTyp {
    if let Yaml::Hash(a) = arg {
        if let Some(Yaml::String(typ)) = a.get(&Yaml::String("type".to_string())) {
            if typ.as_str() == "windmill_variable" {
                if let Some(Yaml::String(variable_path)) =
                    a.get(&Yaml::String("variable".to_string()))
                {
                    return ArgTyp::StaticVar(variable_path.to_string());
                }
            }
            if typ.as_str() == "windmill_resource" {
                if let Some(Yaml::String(resource_path)) =
                    a.get(&Yaml::String("resource".to_string()))
                {
                    return ArgTyp::StaticResource(resource_path.to_string());
                }
            }
        }
    }
    ArgTyp::Typ(parse_ansible_typ(arg))
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

#[derive(Debug, Clone)]
pub struct AnsiblePlaybookOptions {
    // There are a lot more options
    // TODO: Add the options as customers require them
    // Add it here and then on the executors cmd_options
    pub verbosity: Option<String>,
    pub forks: Option<i64>,
    pub timeout: Option<i64>,
    pub flush_cache: Option<()>,
    pub force_handlers: Option<()>,
}

#[derive(Debug, Clone)]
pub enum ResourceOrVariablePath {
    Resource(String),
    Variable(String),
}

#[derive(Debug, Clone)]
pub struct FileResource {
    pub resource_path: ResourceOrVariablePath,
    pub target_path: String,
    pub mode: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct AnsibleInventory {
    pub default: Option<String>,
    pub name: String,
    resource_type: Option<String>,
    pub pinned_resource: Option<String>,
}

#[derive(Debug, Clone)]
pub struct GitRepo {
    pub url: String,
    pub commit: Option<String>,
    pub branch: Option<String>,
    pub target_path: String,
}

#[derive(Debug, Clone)]
pub struct AnsibleRequirements {
    pub python_reqs: Vec<String>,
    pub roles_and_collections: Option<String>,
    pub file_resources: Vec<FileResource>,
    pub inventories: Vec<AnsibleInventory>,
    pub vars: Vec<(String, String)>,
    pub resources: Vec<(String, String)>,
    pub options: AnsiblePlaybookOptions,
    pub vault_password: Option<String>,
    pub vault_id: Vec<String>,
    pub git_repos: Vec<GitRepo>,
    pub git_ssh_identity_files: Vec<String>,
}

impl Default for AnsibleRequirements {
    fn default() -> Self {
        Self {
            python_reqs: vec![],
            roles_and_collections: None,
            file_resources: vec![],
            inventories: vec![],
            vars: vec![],
            resources: vec![],
            options: AnsiblePlaybookOptions {
                verbosity: None,
                forks: None,
                timeout: None,
                flush_cache: None,
                force_handlers: None,
            },
            vault_password: None,
            vault_id: vec![],
            git_repos: vec![],
            git_ssh_identity_files: vec![],
        }
    }
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

    let mut ret = AnsibleRequirements::default();

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
                            ret.roles_and_collections = Some(out_str);
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
                Yaml::String(key) if key == "files" || key == "file_resources" => {
                    if let Yaml::Array(file_resources) = value {
                        let resources: anyhow::Result<Vec<FileResource>> =
                            file_resources.iter().map(parse_file_resource).collect();
                        ret.file_resources.append(&mut resources?);
                    }
                }
                Yaml::String(key) if key == "extra_vars" => {
                    if let Yaml::Hash(v) = &value {
                        for (key, arg) in v {
                            if let Yaml::String(arg_name) = key {
                                match parse_ansible_arg_typ(arg) {
                                    ArgTyp::StaticVar(p) => {
                                        ret.vars
                                            .push((arg_name.to_string(), format!("$var:{}", p)));
                                    }
                                    ArgTyp::StaticResource(p) => {
                                        ret.resources
                                            .push((arg_name.to_string(), format!("$res:{}", p)));
                                    }
                                    ArgTyp::Typ(_) => (),
                                }
                            }
                        }
                    }
                }
                Yaml::String(key) if key == "inventory" => {
                    ret.inventories = parse_inventories(value)?;
                }
                Yaml::String(key) if key == "vault_password" => {
                    let Yaml::String(filename) = value else {
                        return Err(anyhow!(
                            "Vault Password File expects a String containing the file name"
                        ));
                    };
                    ret.vault_password = Some(filename.to_string());
                }
                Yaml::String(key) if key == "vault_id" => {
                    let Yaml::Array(filenames) = value else {
                        return Err(anyhow!("Vault ID field expects an array of strings in the format: `label@filename`"));
                    };

                    for f in filenames {
                        let Yaml::String(filename) = f else {
                            return Err(anyhow!("The elements of the vault_id field should be strings in the format: `label@filename`"));
                        };
                        ret.vault_id.push(filename.to_string());
                    }
                }
                Yaml::String(key) if key == "options" => {
                    if let Yaml::Array(opts) = &value {
                        ret.options = parse_ansible_options(opts);
                    }
                }
                Yaml::String(key) if key == "git_repos" => {
                    let Yaml::Array(repos) = &value else {
                        return Err(anyhow!("git_repos field expects an array of repos"));
                    };

                    for r in repos {
                        ret.git_repos.push(
                            parse_git_repo(r)
                                .map_err(|e| anyhow!("Failed to parse git repo: {e}"))?,
                        );
                    }
                }
                Yaml::String(key) if key == "git_ssh_identity" => {
                    let Yaml::Array(indentity_files) = &value else {
                        return Err(anyhow!(
                            "git_ssh_identity_files expects an array of windmill variables containing ssh IDs"
                        ));
                    };

                    for r in indentity_files {
                        let Yaml::String(file_name) = r else {
                            return Err(anyhow!(
                                "Git ssh identity file must be a string path to a Windmill variable"
                            ));
                        };

                        ret.git_ssh_identity_files.push(file_name.clone());
                    }
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

fn parse_git_repo(r: &Yaml) -> anyhow::Result<GitRepo> {
    let Yaml::Hash(repo) = r else {
        return Err(anyhow!("Should be a Map"));
    };

    let url = repo
        .get(&Yaml::String("url".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(anyhow!("Expected `url` field"))?;

    let target_path = repo
        .get(&Yaml::String("target".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or(anyhow!(
            "Expected `target` field (target directory for cloning the repo)"
        ))?;

    let branch = repo
        .get(&Yaml::String("branch".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let commit = repo
        .get(&Yaml::String("commit".to_string()))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    Ok(GitRepo { url, commit, branch, target_path })
}

fn parse_ansible_options(opts: &Vec<Yaml>) -> AnsiblePlaybookOptions {
    let mut ret = AnsiblePlaybookOptions {
        verbosity: None,
        forks: None,
        timeout: None,
        flush_cache: None,
        force_handlers: None,
    };
    for opt in opts {
        if let Yaml::String(o) = opt {
            match o.as_str() {
                "flush_cache" => {
                    ret.flush_cache = Some(());
                }
                "force_handlers" => {
                    ret.force_handlers = Some(());
                }
                s if count_consecutive_vs(s) > 0 => {
                    ret.verbosity = Some("v".repeat(count_consecutive_vs(s).min(6)));
                }
                _ => (),
            }
        }
        if let Yaml::Hash(m) = opt {
            if let Some((Yaml::String(name), value)) = m.iter().last() {
                match name.as_str() {
                    "forks" => {
                        if let Yaml::Integer(count) = value {
                            ret.forks = Some(*count);
                        }
                    }
                    "timeout" => {
                        if let Yaml::Integer(timeout) = value {
                            ret.timeout = Some(*timeout);
                        }
                    }
                    "verbosity" => {
                        if let Yaml::String(verbosity) = value {
                            let c = count_consecutive_vs(verbosity);
                            if c > 0 && c <= 6 {
                                ret.verbosity = Some("v".repeat(c.min(6)));
                            }
                        }
                    }
                    _ => (),
                }
            }
        }
    }

    ret
}

fn count_consecutive_vs(s: &str) -> usize {
    let mut max_count = 0;
    let mut current_count = 0;

    for c in s.chars() {
        if c == 'v' {
            current_count += 1;
            if current_count == 6 {
                return 6; // Stop early if we reach 6
            }
        } else {
            current_count = 0; // Reset count if the character is not 'v'
        }
        max_count = max_count.max(current_count);
    }

    max_count
}

fn parse_file_resource(yaml: &Yaml) -> anyhow::Result<FileResource> {
    if let Yaml::Hash(f) = yaml {
        let target_path = f
            .get(&Yaml::String("target".to_string()))
            .and_then(|x| x.as_str())
            .map(|x| x.to_string())
            .ok_or(anyhow!(
                "No `target` provided for the file. Please input a target path (only relative paths are allowed) where the ansible playbook can read this file.",
            ))?;

        let mut mode = None;
        if let Some(u) = f.get(&Yaml::String("mode".to_string())) {
            let mode_val: u32 = match u {
                Yaml::Integer(u) => {
                    u.clone().try_into().map_err(|e| {
                        anyhow!(
                            "Invalid value for `mode` permissions property on targeted file to: {}, err: {e}",
                            target_path
                        )
                    })?
                }
                Yaml::String(s) => {
                    let val = if s.starts_with("0b") {
                        u32::from_str_radix(&s[2..], 2)
                    } else if s.starts_with("0o") {
                        u32::from_str_radix(&s[2..], 8)
                    } else if s.starts_with("0") {
                        u32::from_str_radix(&s[1..], 8)
                    } else {
                        u32::from_str_radix(s, 8)
                    };

                    val.map_err(|e| anyhow!("Error parsing permission mode value {s}: {e}"))?
                }
                _ => {
                    return Err(anyhow!("Invalid field in `mode`, expected integer like 0o644"));
                }
            };
            if mode_val <= 0o777 {
                mode = Some(mode_val);
            } else {
                return Err(anyhow!("The provided value for `mode` is too big. Make sure that you are using the octal prefix (0o), e.g. `mode: 0o644`"));
            }
        }

        if let Some(Yaml::String(resource_path)) = f.get(&Yaml::String("resource".to_string())) {
            return Ok(FileResource {
                resource_path: ResourceOrVariablePath::Resource(resource_path.clone()),
                target_path,
                mode,
            });
        }
        if let Some(Yaml::String(resource_path)) = f.get(&Yaml::String("variable".to_string())) {
            return Ok(FileResource {
                resource_path: ResourceOrVariablePath::Variable(resource_path.clone()),
                target_path,
                mode,
            });
        }
        return Err(anyhow!(
            "Files should have a `resource` or `variable` field that will be the contents of the local text file"
        ));
    }
    return Err(anyhow!("Invalid file resource: Should be a dictionary."));
}

fn yaml_to_json(yaml: &Yaml) -> serde_json::Value {
    match yaml {
        Yaml::Array(arr) => {
            let json_array: Vec<serde_json::Value> = arr.into_iter().map(yaml_to_json).collect();
            serde_json::Value::Array(json_array)
        }
        Yaml::Hash(hash) => {
            let json_object = hash
                .into_iter()
                .map(|(k, v)| {
                    let key = match k {
                        Yaml::String(s) => s.clone(),
                        _ => k.as_str().unwrap_or("").to_string(),
                    };
                    (key, yaml_to_json(v))
                })
                .collect();
            serde_json::Value::Object(json_object)
        }
        Yaml::String(s) => serde_json::Value::String(s.to_string()),
        Yaml::Integer(i) => serde_json::Value::Number(i.clone().into()),
        Yaml::Real(r) => serde_json::Value::Number(r.parse().unwrap_or(0.into())),
        Yaml::Boolean(b) => serde_json::Value::Bool(*b),
        Yaml::Null => serde_json::Value::Null,
        _ => serde_json::Value::Null,
    }
}

fn update_versions(
    section: &str,
    yaml: &mut Yaml,
    versions: &HashMap<String, String>,
) -> anyhow::Result<String> {
    let mut logs = String::new();

    let Yaml::Hash(ref mut m) = yaml else {
        return Err(anyhow!("{section} dependency should be a map"));
    };

    if let Some(Yaml::Array(elements)) = m.get_mut(&Yaml::String(section.to_string())) {
        for el in elements {
            let Yaml::Hash(ref mut h) = el else {
                return Err(anyhow!("{section} dependency element should be a map"));
            };

            if let Some(name) = h
                .get(&Yaml::String("name".to_string()))
                .and_then(|n| n.as_str())
            {
                if let Some(version) = versions.get(name) {
                    h.insert(
                        Yaml::String("version".to_string()),
                        Yaml::String(version.to_string()),
                    );
                } else {
                    logs.push_str(&format!("WARNING: {section} dependency `{name}` has no locked version, using the latest or system installed version.\n"));
                }
            } else {
                return Err(anyhow!(
                    "{section} dependency element: missing or invalid `name` field"
                ));
            }
        }
    }

    Ok(logs)
}

pub fn add_versions_to_requirements_yaml(
    input: &str,
    role_versions: &HashMap<String, String>,
    collection_versions: &HashMap<String, String>,
) -> anyhow::Result<(String,String)> {
    let mut docs =
        YamlLoader::load_from_str(input).map_err(|e| anyhow!("YAML parse error: {}", e))?;
    let doc = &mut docs[0];

    let mut logs = String::new();

    logs.push_str(
        &update_versions("roles", doc, role_versions)
            .map_err(|e| anyhow!("Error updating role versions: {e}"))?,
    );
    logs.push_str(
        &update_versions("collections", doc, collection_versions)
            .map_err(|e| anyhow!("Error updating collection versions: {e}"))?,
    );

    if !logs.is_empty() {
        logs.push_str("WARNING: You might want to try adding manual versions for these, otherwise there could be breaking changes on deployed scripts\n");
    }

    let mut out_str = String::new();
    {
        let mut emitter = YamlEmitter::new(&mut out_str);
        emitter
            .dump(doc)
            .map_err(|e| anyhow!("YAML emit error: {}", e))?;
    }

    Ok((out_str, logs))
}
