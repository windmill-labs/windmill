mod tests {

    use sqlx::{Pool, Postgres};
    use windmill_parser_py_imports::{
        parse_code_for_imports, parse_python_imports, ImportPin, NImport,
    };

    #[sqlx::test(fixtures("base"))]
    async fn test_parse_python_imports(db: Pool<Postgres>) -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "

import os
import wmill
from zanzibar.estonie import talin
import matplotlib.pyplot as plt
from . import tests

def main():
    pass

";
        let (r, ..) = parse_python_imports(
            code,
            "test-workspace",
            "f/foo/bar",
            &db,
            &mut vec![],
            &mut None,
            &None,
            &None,
        )
        .await?;
        // println!("{}", serde_json::to_string(&r)?);
        assert_eq!(
            r,
            vec![
                "matplotlib # (mapped from matplotlib.pyplot)",
                "wmill",
                "zanzibar # (mapped from zanzibar.estonie)"
            ]
        );

        Ok(())
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_parse_python_imports2(db: Pool<Postgres>) -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "
#requirements:
#burkina=0.4
#nigeria
#
#congo

import os
import wmill
from zanzibar.estonie import talin

def main():
    pass

";
        let (r, ..) = parse_python_imports(
            code,
            "test-workspace",
            "f/foo/bar",
            &db,
            &mut vec![],
            &mut None,
            &None,
            &None,
        )
        .await?;
        println!("{}", serde_json::to_string(&r)?);
        assert_eq!(r, vec!["burkina=0.4", "nigeria"]);

        Ok(())
    }

    #[sqlx::test(fixtures("base"))]
    async fn test_parse_python_imports_local(db: Pool<Postgres>) -> anyhow::Result<()> {
        //let code = "print(2 + 3, fd=sys.stderr)";
        let code = "
from f.foo.bar import main1
from .baz import main2
from ..foo.bar import main3
import f.foo.bar as bar
from ..foobar.baz import main4

def main():
    pass

";

        let (r, ..) = parse_python_imports(
            code,
            "test-workspace",
            "f/foo/bar",
            &db,
            &mut vec![],
            &mut None,
            &None,
            &None,
        )
        .await?;
        println!("{}", serde_json::to_string(&r)?);
        assert_eq!(
            r,
            [
                "difffolder",
                "innerdifffolder",
                "numpy",
                "pandas",
                "pandas2",
            ]
        );

        Ok(())
    }

    fn auto(pkg: &str) -> NImport {
        NImport::Auto { pkg: pkg.to_string(), key: None }
    }

    #[test]
    fn test_parse_lazy_imports() -> anyhow::Result<()> {
        let code = "
lazy import requests
lazy import numpy # pin: numpy==2.1.0
lazy from pandas import DataFrame
lazy from f.shared import helper
lazy = 1

def main():
    pass
";
        assert_eq!(
            parse_code_for_imports(code, "f/foo/bar")?,
            vec![
                NImport::Pin {
                    pins: vec![ImportPin {
                        pkg: "numpy==2.1.0".to_string(),
                        path: "f/foo/bar".to_string()
                    }],
                    key: "numpy".to_string()
                },
                auto("pandas"),
                auto("requests"),
                NImport::Relative("f/shared".to_string()),
            ]
        );
        Ok(())
    }

    #[test]
    fn test_parse_imports_recent_syntax_before_main() -> anyhow::Result<()> {
        let code = r#"
import requests

type Alias = list[int]
NAME = "x"
D = {"a": 1}
L = [[1], [2]]
TEMPLATE = t"hello {NAME}"
PEP701 = f"{D["a"]} {'\n'.join(["x"])}"
PEP798 = [*a for a in L]

try:
    import ujson
except ImportError, ValueError:
    pass

def main():
    pass
"#;
        assert_eq!(
            parse_code_for_imports(code, "f/foo/bar")?,
            vec![auto("requests")]
        );
        Ok(())
    }

    // Only what precedes `main` is parsed, including when `main` takes type parameters.
    #[test]
    fn test_parse_imports_stops_at_generic_main() -> anyhow::Result<()> {
        for def_main in ["def main[T](x: T):", "async def main[T](x: T):"] {
            let code = format!("import requests\n\n{def_main}\n    return not valid python\n");
            assert_eq!(
                parse_code_for_imports(&code, "f/foo/bar")?,
                vec![auto("requests")]
            );
        }
        Ok(())
    }
}
