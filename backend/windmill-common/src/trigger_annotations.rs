//! Parse `#trigger: asset ...` annotations out of script source.
//!
//! This is the source-of-truth for implicit asset triggers. A script's
//! annotations are re-parsed on every save and projected into rows in the
//! `asset_trigger` table.

use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::{assets::AssetKind, scripts::ScriptLang};

#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum TriggerAnnotationError {
    #[error("trigger annotation is not valid: {0}")]
    Invalid(String),
    #[error("trigger option `{key}` has unsupported value `{value}`: {reason}")]
    UnsupportedValue { key: String, value: String, reason: String },
    #[error("unknown trigger option `{0}`")]
    UnknownOption(String),
    #[error("script has more than one `trigger: asset` annotation — v1 supports a single asset trigger per script")]
    MultipleAssetAnnotations,
}

pub type Result<T> = std::result::Result<T, TriggerAnnotationError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AssetTriggerEvent {
    Change,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FiresMode {
    All,
    Any,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BacklogMode {
    Coalesce,
    Replay,
    Skip,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "lowercase")]
pub enum PartitionMap {
    Identity,
    None,
    All,
    Window { back_days: i32, forward_days: i32 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AssetTriggerAnnotation {
    pub on: AssetTriggerEvent,
    pub fires: FiresMode,
    pub debounce_s: i32,
    /// If `Some`, only these upstream paths (filtered by kinds) are subscribed
    /// to; overrides the script's inferred read set.
    pub only: Option<Vec<String>>,
    pub exclude: Vec<String>,
    pub extra: Vec<String>,
    /// If `Some`, the subscription set is narrowed to these asset kinds.
    pub kinds: Option<Vec<AssetKind>>,
    pub partition_map: PartitionMap,
    pub cancel_on_new: bool,
    pub backlog: BacklogMode,
}

impl Default for AssetTriggerAnnotation {
    fn default() -> Self {
        Self {
            on: AssetTriggerEvent::Change,
            fires: FiresMode::All,
            debounce_s: 30,
            only: None,
            exclude: Vec::new(),
            extra: Vec::new(),
            kinds: None,
            partition_map: PartitionMap::Identity,
            cancel_on_new: false,
            backlog: BacklogMode::Coalesce,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum TriggerAnnotation {
    Asset(AssetTriggerAnnotation),
}

/// Parse every `<comment> trigger: ...` annotation found anywhere in the file.
///
/// Returns `Ok(vec![])` when no trigger annotation is present — this is the
/// common case. Errors are only returned for malformed annotations so a
/// missing trigger never blocks a save.
pub fn parse_trigger_annotations(code: &str, lang: ScriptLang) -> Result<Vec<TriggerAnnotation>> {
    let prefix = lang.as_comment_lit();
    let marker = format!("{prefix} trigger:");
    let mut out = Vec::new();
    let mut asset_seen = false;
    for raw in code.lines() {
        let line = raw.trim_end();
        let trimmed = line.trim_start();
        let Some(rest) = trimmed.strip_prefix(&marker) else {
            continue;
        };
        let rest = rest.trim();
        if let Some(body) = rest.strip_prefix("asset").map(|r| r.trim()) {
            if asset_seen {
                return Err(TriggerAnnotationError::MultipleAssetAnnotations);
            }
            asset_seen = true;
            out.push(TriggerAnnotation::Asset(parse_asset_body(body)?));
        } else {
            return Err(TriggerAnnotationError::Invalid(format!(
                "expected `trigger: asset ...`, got `trigger: {rest}`"
            )));
        }
    }
    Ok(out)
}

fn parse_asset_body(body: &str) -> Result<AssetTriggerAnnotation> {
    let mut ann = AssetTriggerAnnotation::default();
    for token in split_top_level(body)? {
        let Some((key, value)) = token.split_once('=') else {
            return Err(TriggerAnnotationError::Invalid(format!(
                "expected `key=value`, got `{token}`"
            )));
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "on" => {
                if value != "change" {
                    return Err(TriggerAnnotationError::UnsupportedValue {
                        key: key.to_string(),
                        value: value.to_string(),
                        reason: "only `on=change` is supported in v1".to_string(),
                    });
                }
                ann.on = AssetTriggerEvent::Change;
            }
            "fires" => {
                ann.fires = match value {
                    "all" => FiresMode::All,
                    "any" => FiresMode::Any,
                    other => {
                        return Err(TriggerAnnotationError::UnsupportedValue {
                            key: key.to_string(),
                            value: other.to_string(),
                            reason: "expected `all` or `any`".to_string(),
                        });
                    }
                };
            }
            "debounce" => {
                ann.debounce_s = parse_duration_seconds(value).map_err(|e| {
                    TriggerAnnotationError::UnsupportedValue {
                        key: key.to_string(),
                        value: value.to_string(),
                        reason: e,
                    }
                })?;
            }
            "only" => ann.only = Some(parse_list(value)),
            "exclude" => ann.exclude = parse_list(value),
            "extra" => ann.extra = parse_list(value),
            "kinds" => {
                let list = parse_list(value);
                let mut kinds = Vec::with_capacity(list.len());
                for k in list {
                    kinds.push(parse_asset_kind(&k).ok_or_else(|| {
                        TriggerAnnotationError::UnsupportedValue {
                            key: "kinds".to_string(),
                            value: k.clone(),
                            reason: "expected one of datatable|ducklake|s3object|resource|volume"
                                .to_string(),
                        }
                    })?);
                }
                ann.kinds = Some(kinds);
            }
            "partition_map" => {
                ann.partition_map = parse_partition_map(value).map_err(|reason| {
                    TriggerAnnotationError::UnsupportedValue {
                        key: key.to_string(),
                        value: value.to_string(),
                        reason,
                    }
                })?;
            }
            "cancel_on_new" => {
                ann.cancel_on_new =
                    parse_bool(value).ok_or_else(|| TriggerAnnotationError::UnsupportedValue {
                        key: key.to_string(),
                        value: value.to_string(),
                        reason: "expected `true` or `false`".to_string(),
                    })?;
            }
            "backlog" => {
                ann.backlog = match value {
                    "coalesce" => BacklogMode::Coalesce,
                    "replay" => BacklogMode::Replay,
                    "skip" => BacklogMode::Skip,
                    other => {
                        return Err(TriggerAnnotationError::UnsupportedValue {
                            key: key.to_string(),
                            value: other.to_string(),
                            reason: "expected `coalesce`, `replay`, or `skip`".to_string(),
                        });
                    }
                };
            }
            other => return Err(TriggerAnnotationError::UnknownOption(other.to_string())),
        }
    }
    Ok(ann)
}

/// Split a body like `on=change fires=all only=[a,b] debounce=30s` into
/// tokens, preserving bracketed list values as a single token.
fn split_top_level(body: &str) -> Result<Vec<String>> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut depth: i32 = 0;
    for ch in body.chars() {
        match ch {
            '[' => {
                depth += 1;
                current.push(ch);
            }
            ']' => {
                depth -= 1;
                if depth < 0 {
                    return Err(TriggerAnnotationError::Invalid(
                        "unbalanced `]` in trigger annotation".to_string(),
                    ));
                }
                current.push(ch);
            }
            c if c.is_whitespace() && depth == 0 => {
                if !current.is_empty() {
                    tokens.push(std::mem::take(&mut current));
                }
            }
            _ => current.push(ch),
        }
    }
    if depth != 0 {
        return Err(TriggerAnnotationError::Invalid(
            "unbalanced `[` in trigger annotation".to_string(),
        ));
    }
    if !current.is_empty() {
        tokens.push(current);
    }
    Ok(tokens)
}

fn parse_list(value: &str) -> Vec<String> {
    let trimmed = value.trim();
    let inner = trimmed
        .strip_prefix('[')
        .and_then(|s| s.strip_suffix(']'))
        .unwrap_or(trimmed);
    inner
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_bool(value: &str) -> Option<bool> {
    match value {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

fn parse_asset_kind(s: &str) -> Option<AssetKind> {
    match s {
        "s3object" => Some(AssetKind::S3Object),
        "resource" => Some(AssetKind::Resource),
        "variable" => Some(AssetKind::Variable),
        "ducklake" => Some(AssetKind::Ducklake),
        "datatable" => Some(AssetKind::DataTable),
        "volume" => Some(AssetKind::Volume),
        _ => None,
    }
}

/// Parse durations like `30s`, `5m`, `2h`, `1d`. Returns whole seconds.
/// Bare integers are accepted as seconds for backward-compatibility with
/// simple `debounce=30`.
fn parse_duration_seconds(s: &str) -> std::result::Result<i32, String> {
    let s = s.trim();
    if s.is_empty() {
        return Err("empty duration".to_string());
    }
    let (num, unit) = match s
        .char_indices()
        .find(|(_, c)| !c.is_ascii_digit() && *c != '-')
    {
        Some((i, _)) => (&s[..i], &s[i..]),
        None => (s, "s"),
    };
    let n: i32 = i32::from_str(num).map_err(|_| format!("not a number: `{num}`"))?;
    let mult = match unit {
        "s" | "" => 1,
        "m" => 60,
        "h" => 60 * 60,
        "d" => 24 * 60 * 60,
        other => return Err(format!("unknown duration unit `{other}`")),
    };
    Ok(n * mult)
}

fn parse_partition_map(value: &str) -> std::result::Result<PartitionMap, String> {
    match value {
        "identity" => Ok(PartitionMap::Identity),
        "none" => Ok(PartitionMap::None),
        "all" => Ok(PartitionMap::All),
        other if other.starts_with("window(") && other.ends_with(')') => {
            let inside = &other["window(".len()..other.len() - 1];
            let mut parts = inside.split(',').map(str::trim);
            let back = parts
                .next()
                .ok_or_else(|| "window() needs two arguments".to_string())?;
            let forward = parts
                .next()
                .ok_or_else(|| "window() needs two arguments".to_string())?;
            if parts.next().is_some() {
                return Err("window() takes exactly two arguments".to_string());
            }
            let back_days = parse_signed_days(back)?;
            let forward_days = parse_signed_days(forward)?;
            Ok(PartitionMap::Window { back_days, forward_days })
        }
        other => Err(format!(
            "expected `identity`, `none`, `all`, or `window(-Nd,Md)`, got `{other}`"
        )),
    }
}

fn parse_signed_days(s: &str) -> std::result::Result<i32, String> {
    let s = s.trim();
    let rest = s
        .strip_suffix('d')
        .ok_or_else(|| format!("expected a signed day literal like `-7d` or `3d`, got `{s}`"))?;
    i32::from_str(rest).map_err(|_| format!("not a number: `{rest}`"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_none_when_no_annotation() {
        let out =
            parse_trigger_annotations("let x = 1;\n// just a comment", ScriptLang::Bun).unwrap();
        assert!(out.is_empty());
    }

    #[test]
    fn parses_minimal_python_annotation() {
        let src = "# trigger: asset on=change\nprint('hi')";
        let out = parse_trigger_annotations(src, ScriptLang::Python3).unwrap();
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.on, AssetTriggerEvent::Change);
        assert_eq!(ann.fires, FiresMode::All);
        assert_eq!(ann.debounce_s, 30);
        assert_eq!(ann.cancel_on_new, false);
        assert_eq!(ann.backlog, BacklogMode::Coalesce);
        assert_eq!(ann.partition_map, PartitionMap::Identity);
    }

    #[test]
    fn parses_bun_annotation_anywhere_in_file() {
        let src = "import x from 'y';\n\n// trigger: asset on=change fires=any\nexport async function main() {}";
        let out = parse_trigger_annotations(src, ScriptLang::Bun).unwrap();
        assert_eq!(out.len(), 1);
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.fires, FiresMode::Any);
    }

    #[test]
    fn parses_sql_annotation() {
        let src = "-- trigger: asset on=change debounce=5m\nSELECT 1;";
        let out = parse_trigger_annotations(src, ScriptLang::Postgresql).unwrap();
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.debounce_s, 300);
    }

    #[test]
    fn parses_rust_annotation_with_its_bespoke_prefix() {
        let src = "//! trigger: asset on=change backlog=skip\nfn main() {}";
        let out = parse_trigger_annotations(src, ScriptLang::Rust).unwrap();
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.backlog, BacklogMode::Skip);
    }

    #[test]
    fn parses_full_option_set() {
        let src = "# trigger: asset on=change fires=any debounce=2h only=[f/a,f/b] exclude=[f/c] extra=[f/d] kinds=[datatable,s3object] partition_map=window(-7d,0d) cancel_on_new=true backlog=replay\n";
        let out = parse_trigger_annotations(src, ScriptLang::Python3).unwrap();
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.fires, FiresMode::Any);
        assert_eq!(ann.debounce_s, 2 * 60 * 60);
        assert_eq!(ann.only, Some(vec!["f/a".to_string(), "f/b".to_string()]));
        assert_eq!(ann.exclude, vec!["f/c".to_string()]);
        assert_eq!(ann.extra, vec!["f/d".to_string()]);
        assert_eq!(
            ann.kinds,
            Some(vec![AssetKind::DataTable, AssetKind::S3Object])
        );
        assert_eq!(
            ann.partition_map,
            PartitionMap::Window { back_days: -7, forward_days: 0 }
        );
        assert_eq!(ann.cancel_on_new, true);
        assert_eq!(ann.backlog, BacklogMode::Replay);
    }

    #[test]
    fn rejects_unsupported_on_value() {
        let src = "# trigger: asset on=freshness\n";
        let err = parse_trigger_annotations(src, ScriptLang::Python3).unwrap_err();
        assert!(matches!(
            err,
            TriggerAnnotationError::UnsupportedValue { .. }
        ));
    }

    #[test]
    fn rejects_duplicate_asset_annotation() {
        let src = "# trigger: asset on=change\n# trigger: asset on=change\n";
        let err = parse_trigger_annotations(src, ScriptLang::Python3).unwrap_err();
        assert_eq!(err, TriggerAnnotationError::MultipleAssetAnnotations);
    }

    #[test]
    fn rejects_unknown_option() {
        let src = "# trigger: asset on=change zombie=true\n";
        let err = parse_trigger_annotations(src, ScriptLang::Python3).unwrap_err();
        assert!(matches!(err, TriggerAnnotationError::UnknownOption(o) if o == "zombie"));
    }

    #[test]
    fn rejects_malformed_kind() {
        let src = "# trigger: asset on=change kinds=[s3object,foobar]\n";
        let err = parse_trigger_annotations(src, ScriptLang::Python3).unwrap_err();
        assert!(
            matches!(err, TriggerAnnotationError::UnsupportedValue { key, .. } if key == "kinds")
        );
    }

    #[test]
    fn rejects_bad_partition_window() {
        let src = "# trigger: asset on=change partition_map=window(abc,def)\n";
        let err = parse_trigger_annotations(src, ScriptLang::Python3).unwrap_err();
        assert!(
            matches!(err, TriggerAnnotationError::UnsupportedValue { key, .. } if key == "partition_map")
        );
    }

    #[test]
    fn accepts_bare_integer_debounce_as_seconds() {
        let src = "# trigger: asset on=change debounce=45\n";
        let out = parse_trigger_annotations(src, ScriptLang::Python3).unwrap();
        let TriggerAnnotation::Asset(ann) = &out[0];
        assert_eq!(ann.debounce_s, 45);
    }

    #[test]
    fn ignores_lines_with_other_comment_prefix() {
        // `# trigger: ...` in a bun file uses `//` as prefix, so a `#` line is ignored.
        let src = "# trigger: asset on=change\nconsole.log('hi');";
        let out = parse_trigger_annotations(src, ScriptLang::Bun).unwrap();
        assert!(out.is_empty());
    }
}
