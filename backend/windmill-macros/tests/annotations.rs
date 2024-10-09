use std::time::Instant;

// TODO: Make sure these are being tested if we run cargo test in root
// TODO: Remove next 2 lines?
#[cfg(test)]
mod annotations_tests {

    extern crate windmill_macros;

    use std::time::Instant;

    // use annotations;
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
        let cont = "// ann1
// ann2";
        assert_eq!(
            SlashedAnnotations { ann1: true, ann2: true },
            SlashedAnnotations::parse(cont)
        );
    }

    // e.g. Haskell, SQL
    #[test]
    fn minused_annotations() {
        let cont = "-- ann1
-- ann2";
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

    #[annotations("#")]
    #[derive(Eq, PartialEq, Copy, Clone)]
    pub struct BenchAnnotations {
        pub ann1: bool,
        pub ann2: bool,
        pub ann3: bool,
        pub ann4: bool,
        pub ann5: bool,
        pub ann6: bool,
        pub ann7: bool,
        pub ann8: bool,
        pub ann9: bool,
        pub ann10: bool,
        pub ann11: bool,
        pub ann12: bool,
        pub ann13: bool,
        pub ann14: bool,
        pub ann15: bool,
        pub ann16: bool,
        pub ann17: bool,
        pub ann18: bool,
        pub ann19: bool,
        pub ann20: bool,
        pub ann21: bool,
        pub ann22: bool,
        pub ann23: bool,
        pub ann24: bool,
        pub ann25: bool,
        pub ann26: bool,
        pub ann27: bool,
        pub ann28: bool,
        pub ann29: bool,
        pub ann30: bool,
        pub ann31: bool,
        pub ann32: bool,
        pub ann33: bool,
        pub ann34: bool,
    }
    fn bench_legacy(inner_content: &str) {
        let annotations = inner_content
            .lines()
            .take_while(|x| x.starts_with("#"))
            .map(|x| x.to_string().replace("#", "").trim().to_string())
            .collect_vec();

        for i in 1..35 {
            let ann_name = format!("ann{}", i);
            let _ = annotations.contains(&ann_name);
        }
    }
    // TODO: Move bench somewhere else and refactor
    #[test]
    fn bench() {
        let cont = "# ann2
# ann5
# ann3
# ann4
# ann5
# ann6
# ann7
# ann8
# ann9
# ann10
# ann11
# ann12
# ann19
# ann20
# ann21
# ann22
# ann25
# ann23
# ann24
# ann26
# ann27
# ann28
# ann29
#
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# ann30
            ";
        let cont2 = "# ann2 ann5 ann3 ann4 
# ann5 ann6 ann7 ann8 ann9 ann10
# ann11 ann12 ann19 ann20 ann21 ann22 
# ann25 ann23 ann24 ann26 ann27 ann28 ann29
#
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# Lorem ipsum dolor sit amet, consectetur adipiscing elit. 
# Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. 
# Ut enim ad minim veniam, quis nostrud exercitation 
# ullamco laboris nisi ut aliquip ex ea commodo consequat.
# ann30
            ";

        {
            let start = Instant::now();
            for _ in 0..10000 {
                BenchAnnotations::parse(cont2);
            }
            let end = Instant::now();
            let duration = end - start;
            dbg!("New:");
            dbg!(duration);
        }

        {
            let start = Instant::now();
            for _ in 0..10000 {
                bench_legacy(cont);
            }
            let end = Instant::now();
            let duration = end - start;
            dbg!("Old:");
            dbg!(duration);
        }
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
