use itertools::Itertools;

pub struct WorkspaceDependenciesRefs {
    /// ```python
    /// # requirements:
    /// # rich==x.y.z    <<
    /// # pandas==x.y.z  <<
    /// ```
    pub inline: Option<String>,
    /// ```python
    /// # requirements: default, base, prod
    ///                 ^^^^^^^  ^^^^  ^^^^
    /// ```
    pub external: Vec<String>,
    pub mode: Mode,
}

/// `# extra_requirements:` - Extra
/// `# requirements:` - Manual
pub enum Mode {
    Manual,
    Extra,
}

// TODO: Maybe implemented by our Annotations macro
// TODO: Note this will include all seqsequent lines as long as they start with comment.
// TODO: What sep should be? ':' or '='?
pub fn parse_annotation(
    comment: &str,
    keyword: &str,
    code: &str,
) -> Option<WorkspaceDependenciesRefs> {
    let (extra_deps, manual_deps) = (format!("extra_{keyword}:"), format!("{keyword}:"));

    let Some((pos, mat)) = code.lines().find_position(|l| {
        l.starts_with(&comment) && (l.contains(&extra_deps) || l.contains(&manual_deps))
    }) else {
        return None;
    };
    let mut lines_it = code.lines().skip(pos);

    let mode = if mat.contains(&extra_deps) {
        Mode::Extra
    } else {
        Mode::Manual
    };

    let external = lines_it
        .next()
        // We are not interested in matched annotation.
        .map(|s| {
            match mode {
                Mode::Manual => s.replace(&manual_deps, ""),
                Mode::Extra => s.replace(&extra_deps, ""),
            }
            .replace(comment, "")
        })
        .map(|unparsed| {
            unparsed
                .split(',')
                // TODO: do we want to sort it?
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned)
                .collect_vec()
        })
        .unwrap_or_default();

    dbg!(&external);

    let inline_deps = lines_it
        .map_while(|l| {
            if !l.starts_with(comment) {
                None
            } else {
                // Skip comment
                // If it fails (None) iteration is just finished.
                l.get(comment.len()..)
            }
        })
        .join("\n");

    let inline = if inline_deps.trim().is_empty() {
        None
    } else {
        Some(inline_deps)
    };

    Some(WorkspaceDependenciesRefs {
        inline, // TODO: Parse
        external,
        mode,
    })
}

#[cfg(test)]
mod workspace_dependencies_tests {
    use super::*;

    #[test]
    fn test_parse_annotation_python_requirements_manual_mode() {
        let code = r#"
# requirements:  default,   base
#requests==2.31.0
#pandas>=1.5.0

def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(
            result.external,
            vec!["default".to_owned(), "base".to_owned()]
        );
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "requests==2.31.0\npandas>=1.5.0"
        );
    }

    #[test]
    fn test_parse_annotation_python_extra_requirements_mode() {
        let code = r#"
# extra_requirements: utils
#numpy>=1.24.0

def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Extra));
        assert_eq!(result.external, vec!["utils".to_owned()]);
        assert_eq!(result.inline.as_ref().unwrap(), "numpy>=1.24.0");
    }

    #[test]
    fn test_parse_annotation_typescript_requirements() {
        let code = r#"
// requirements: utils, base
//{
//  "dependencies": {
//    "axios": "^1.6.0"
//  }
//}

export function main() {}
"#;

        let result = parse_annotation("//", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["utils".to_owned(), "base".to_owned()]);
        let expected_inline = r#"{
  "dependencies": {
    "axios": "^1.6.0"
  }
}"#;
        assert_eq!(result.inline.as_ref().unwrap(), expected_inline);
    }

    #[test]
    fn test_parse_annotation_with_spacing_variations() {
        let code = r#"
#requirements: no_space
def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["no_space".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_with_spacing_variations_spaced() {
        let code = r#"
# requirements: with_space
def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["with_space".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_go_style() {
        let code = r#"
// requirements:   base,
//github.com/gin-gonic/gin v1.9.1

package main
func main() {}
"#;

        let result = parse_annotation("//", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["base".to_owned()]);
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "github.com/gin-gonic/gin v1.9.1"
        );
    }

    #[test]
    fn test_parse_annotation_no_inline_deps() {
        let code = r#"
# requirements: default

def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["default".to_owned()]);
        assert!(result.inline.is_none());
    }

    #[test]
    fn test_parse_annotation_inline_only() {
        let code = r#"
# requirements:
#requests==2.31.0
# pandas>=1.5.0

def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert!(result.external.is_empty());
        assert_eq!(
            result.inline.as_ref().unwrap(),
            "requests==2.31.0\n pandas>=1.5.0"
        );
    }

    #[test]
    fn test_parse_annotation_no_match() {
        let code = r#"
def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_annotation_php_style() {
        let code = r#"
<?php
// requirements: composer
//{
//  "require": {
//    "guzzlehttp/guzzle": "^7.0"
//  }
//}

function main() {}
"#;

        let result = parse_annotation("//", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert_eq!(result.external, vec!["composer".to_owned()]);
        let expected_inline = r#"{
  "require": {
    "guzzlehttp/guzzle": "^7.0"
  }
}"#;
        assert_eq!(result.inline.as_ref().unwrap(), expected_inline);
    }

    #[test]
    fn test_parse_annotation_just_requirements_colon() {
        let code = r#"
#requirements:

def main():
    pass
"#;

        let result = parse_annotation("#", "requirements", code).unwrap();
        assert!(matches!(result.mode, Mode::Manual));
        assert!(result.external.is_empty());
        assert!(result.inline.is_none());
    }
}
