use windmill_macros::annotations;

// #[derive(Reflect, Default)]
// pub struct PythonAnnotations {
//     pub no_uv: bool,
//     pub no_cache: bool,
// }

// Rules:
// 1. Should be first line
// 2. Every annotation with breaking # will break detection
// 3.

/* Problems:

    Code:
        # we are not using nobundling here
    Legacy:
    - Will be detected as comment and not nobundling annotation
    New:
    - Will detect nobundling as annotation

    Solution:
        1. Make this line invalid and only count lines with all valid annotations:
            # ann, ann2, ann3 and here we go with comments ann4
                ann4 will be ignored and only ann, ann2 and ann3 will be detected
        2. Drop line if anything is not valid annotation



*/
#[annotations("#")]
pub struct PythonAnnotations {
    pub no_cache: bool,
    pub no_uv: bool,
}

#[annotations("//")]
pub struct TSAnnotations {
    pub no_cache: bool,
    pub no_uv: bool,
}

#[annotations("--")]
pub struct SQLAnnotations {
    pub no_cache: bool,
    pub no_uv: bool,
}

// TODO: Move to different module
// TODO: Make sure these are being tested if we run cargo test in root
#[cfg(test)]
mod tests {

    use itertools::Itertools;
    use windmill_macros::annotations;

    // Previous implementation.
    // We have to make sure that new one works the same as old one
    fn old(inner_content: &str) -> Annotations {
        let annotations = inner_content
            .lines()
            .take_while(|x| x.starts_with("#"))
            .map(|x| x.to_string().replace("#", "").trim().to_string())
            .collect_vec();

        let ann1: bool = annotations.contains(&"ann1".to_string());
        let ann2: bool = annotations.contains(&"ann2".to_string());
        let ann3: bool = annotations.contains(&"ann3".to_string());
        let ann4: bool = annotations.contains(&"ann4".to_string());
        let ann5: bool = annotations.contains(&"ann5".to_string());

        Annotations { ann1, ann2, ann3, ann4, ann5 }
    }

    #[annotations("#")]
    #[derive(Eq, PartialEq, Copy, Clone)]
    pub struct Annotations {
        pub ann1: bool,
        pub ann2: bool,
        pub ann3: bool,
        pub ann4: bool,
        pub ann5: bool,
    }

    #[annotations("//")]
    #[derive(Eq, PartialEq, Copy, Clone)]
    pub struct SlashedAnnotations {
        pub ann1: bool,
        pub ann2: bool,
    }

    #[annotations("--")]
    #[derive(Eq, PartialEq, Copy, Clone)]
    pub struct MinusedAnnotations {
        pub ann1: bool,
        pub ann2: bool,
    }

    // e.g. rust, TS and JS
    #[test]
    fn slashed_annotations() {
        let cont = "// ann1, ann2";
        assert_eq!(
            SlashedAnnotations { ann1: true, ann2: true },
            SlashedAnnotations::parse(cont)
        );
    }

    // e.g. Haskell, SQL
    #[test]
    fn minused_annotations() {
        let cont = "-- ann1, ann2";
        assert_eq!(
            MinusedAnnotations { ann1: true, ann2: true },
            MinusedAnnotations::parse(cont)
        );
    }

    #[test]
    fn simple_integration() {
        let cont = "# ann1";
        let expected = Annotations { ann1: true, ..Default::default() };
        assert_eq!(expected, old(cont));
        assert_eq!(expected, Annotations::parse(cont));
    }

    #[test]
    fn multiline_integration() {
        let cont = "# ann2
# ann3
# ann4
# ann5
# ann1
            ";
        let expected = Annotations {
            ann1: true,
            ann2: true,
            ann3: true,
            ann4: true,
            ann5: true,
            //
        };
        assert_eq!(expected, old(cont));
        assert_eq!(expected, Annotations::parse(cont));
    }

    #[test]
    fn spacing_integration() {
        // First line is ignored and not used
        {
            let cont = "
# ann2";
            let expected = Annotations { ..Default::default() };
            assert_eq!(expected, old(cont));
            assert_eq!(expected, Annotations::parse(cont));
        }
        // Wrong spacing for ann3
        {
            let cont = "# ann2
                        # ann3";

            let expected = Annotations { ann2: true, ..Default::default() };
            assert_eq!(expected, old(cont));
            assert_eq!(expected, Annotations::parse(cont));
        }

        // Drunk but valid spacing
        {
            let cont = "#ann1
#                        ann2";
            let expected = Annotations { ann2: true, ann1: true, ..Default::default() };
            assert_eq!(expected, old(cont));
            assert_eq!(expected, Annotations::parse(cont));
        }
    }

    #[test]
    fn comments_inbetween_integration() {
        let cont = "# ann2
# Just comment, has nothing to do with annotations
# Another comment: ann1 ann2 ann3 ann4
# Actual annotation next line:
# ann5

# Should be ignored
# ann3
            ";
        let expected = Annotations { ann2: true, ann5: true, ..Default::default() };
        assert_eq!(expected, old(cont));
        assert_eq!(expected, Annotations::parse(cont));
    }

    #[test]
    fn multiline() {
        let cont = "# ann2
# Just comment, has nothing to do with annotations
# Another comment that should ignore: ann1 ann2 ann3 ann4
# Actual annotation next line:
# ann5
            ";
        assert_eq!(old(cont), Annotations::parse(cont));
    }

    #[test]
    fn non_matching_integration() {
        {
            let cont = r#" "ann1", ann2 "#;
            let expected = Annotations::default();
            assert_eq!(expected, old(cont));
            assert_eq!(expected, Annotations::parse(cont));
        }
        // Empty
        {
            let cont = "";
            let expected = Annotations::default();
            assert_eq!(expected, old(cont));
            assert_eq!(expected, Annotations::parse(cont));
        }
    }
}
