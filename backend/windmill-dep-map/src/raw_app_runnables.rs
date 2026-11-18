use std::collections::{BTreeMap, HashMap, HashSet};

use serde_json::{from_value, Value};
use windmill_common::scripts::{ScriptLang, ScriptModule};

use crate::{extract_referenced_paths, extract_relative_imports};

/// A raw app's inline runnables, keyed by the path a relative import of one resolves to:
/// `<app path>/<runnable id>`, the path the runnable itself runs under. A runnable ships
/// the ones it imports as its modules, so they resolve from the job dir at run time.
#[derive(Default)]
pub struct RawAppRunnables(HashMap<String, (String, ScriptLang, String)>);

impl RawAppRunnables {
    pub fn from_value(app_path: &str, value: &Value) -> Self {
        let mut runnables = HashMap::new();
        if let Some(Value::Object(by_id)) = value.get("runnables") {
            for (id, runnable) in by_id {
                let Some(script) = runnable.get("inlineScript") else {
                    continue;
                };
                let content = script.get("content").and_then(Value::as_str);
                let language = script
                    .get("language")
                    .and_then(|l| from_value::<ScriptLang>(l.clone()).ok());
                if let (Some(content), Some(language)) = (content, language) {
                    // `main.ts` is the Bun entrypoint's own file in the job dir, so a
                    // runnable with that id cannot ship as a module next to it.
                    let ext = module_extension(language);
                    if ext.is_some() && !(ext == Some("ts") && id == "main") {
                        runnables.insert(
                            format!("{app_path}/{id}"),
                            (id.clone(), language, content.to_string()),
                        );
                    }
                }
            }
        }
        Self(runnables)
    }

    /// Whether an import of `path` by `language` code resolves to a runnable shipped as a
    /// module, rather than to the workspace script at that path.
    pub fn resolves(&self, path: &str, language: ScriptLang) -> bool {
        self.0.get(path).is_some_and(|(_, sibling_language, _)| {
            module_extension(*sibling_language) == module_extension(language)
        })
    }

    /// The runnables `content` imports, directly or through one another.
    pub fn modules_of(
        &self,
        app_path: &str,
        content: &str,
        language: ScriptLang,
    ) -> Option<BTreeMap<String, ScriptModule>> {
        let ext = module_extension(language)?;
        let mut queue =
            extract_relative_imports(content, &format!("{app_path}/app"), &Some(language))
                .unwrap_or_default();
        let mut visited = HashSet::new();
        let mut modules = BTreeMap::new();
        while let Some(path) = queue.pop() {
            if !visited.insert(path.clone()) {
                continue;
            }
            let Some((id, sibling_language, sibling_content)) = self.0.get(&path) else {
                continue;
            };
            if module_extension(*sibling_language) != Some(ext) {
                continue;
            }
            queue.extend(
                extract_relative_imports(sibling_content, &path, &Some(*sibling_language))
                    .unwrap_or_default(),
            );
            modules.insert(
                format!("{id}.{ext}"),
                ScriptModule {
                    content: sibling_content.clone(),
                    language: *sibling_language,
                    lock: None,
                },
            );
        }
        (!modules.is_empty()).then_some(modules)
    }

    /// What an app inline script depends on, as its dependency-map node records it. An import
    /// satisfied by another runnable is not a dependency on the workspace script at that path,
    /// but what that runnable imports is: the script's lock covers its modules, so a change
    /// there must relock it and fail its skip check.
    pub fn referenced_paths(
        &self,
        app_path: &str,
        content: &str,
        language: ScriptLang,
    ) -> Option<Vec<String>> {
        let base = format!("{app_path}/app");
        let own = extract_referenced_paths(content, &base, Some(language));
        let Some(modules) = self.modules_of(app_path, content, language) else {
            return own.map(|paths| {
                paths
                    .into_iter()
                    .filter(|p| !self.resolves(p, language))
                    .collect()
            });
        };
        let mut paths = own.unwrap_or_default();
        for module in modules.values() {
            paths.extend(
                extract_referenced_paths(&module.content, &base, Some(module.language))
                    .unwrap_or_default(),
            );
        }
        paths.retain(|p| !self.resolves(p, language));
        paths.sort();
        paths.dedup();
        Some(paths)
    }
}

/// The languages whose executors resolve a relative import against a module file.
pub fn module_extension(language: ScriptLang) -> Option<&'static str> {
    match language {
        ScriptLang::Bun | ScriptLang::Bunnative => Some("ts"),
        ScriptLang::Python3 => Some("py"),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modules_follow_imports_between_runnables_of_one_language() {
        let value = json!({ "runnables": {
            "main": { "inlineScript": { "language": "bun", "content": "import { a } from './a'" } },
            "a": { "inlineScript": { "language": "bun", "content": "import { b } from './b.ts'" } },
            "b": { "inlineScript": { "language": "bun", "content": "export const b = 1" } },
            "unused": { "inlineScript": { "language": "bun", "content": "export const u = 1" } },
            "py": { "inlineScript": { "language": "python3", "content": "x = 1" } },
        }});
        let runnables = RawAppRunnables::from_value("u/admin/app", &value);

        let modules = runnables
            .modules_of(
                "u/admin/app",
                "import { a } from './a'\nimport { p } from './py'",
                ScriptLang::Bun,
            )
            .unwrap();
        assert_eq!(modules.keys().collect::<Vec<_>>(), vec!["a.ts", "b.ts"]);
        assert!(runnables
            .modules_of("u/admin/app", "export const x = 1", ScriptLang::Bun)
            .is_none());
    }
}
