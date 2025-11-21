use crate::scripts::ScriptLang;

pub const LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT: &str = "# from requirements.txt";

pub fn is_generated_from_raw_requirements(
    lang: &Option<ScriptLang>,
    lock: &Option<String>,
) -> bool {
    (lang.is_some_and(|v| v == ScriptLang::Bun)
        && lock
            .as_ref()
            .is_some_and(|v| v.contains("generatedFromPackageJson")))
        || (lang.is_some_and(|v| v == ScriptLang::Python3)
            && lock
                .as_ref()
                .is_some_and(|v| v.starts_with(LOCKFILE_GENERATED_FROM_REQUIREMENTS_TXT)))
}
