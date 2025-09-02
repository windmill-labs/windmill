use rustpython_ast::{Constant, Expr, ExprConstant, Visitor};
use rustpython_parser::{ast::Suite, Parse};
use windmill_parser::asset_parser::{
    merge_assets, parse_asset_syntax, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

pub fn parse_assets(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<String>>> {
    let ast = Suite::parse(input, "main.py")
        .map_err(|e| anyhow::anyhow!("Error parsing code: {}", e.to_string()))?;

    let mut assets_finder = AssetsFinder { assets: vec![] };
    ast.into_iter()
        .for_each(|stmt| assets_finder.visit_stmt(stmt));
    Ok(merge_assets(assets_finder.assets))
}

struct AssetsFinder {
    assets: Vec<ParseAssetsResult<String>>,
}
impl Visitor for AssetsFinder {
    // visit_call_expr will not recurse if it detects an asset,
    // so this will only be called when no further context was found
    fn visit_expr_constant(&mut self, node: ExprConstant) {
        match node.value {
            Constant::Str(s) => {
                if let Some((kind, path)) = parse_asset_syntax(&s) {
                    self.assets.push(ParseAssetsResult {
                        kind,
                        path: path.to_string(),
                        access_type: None,
                    });
                }
            }
            _ => self.generic_visit_expr_constant(node),
        }
    }

    fn visit_expr_call(&mut self, node: rustpython_ast::ExprCall) {
        match self.visit_expr_call_inner(&node) {
            Ok(_) => {}
            Err(_) => self.generic_visit_expr_call(node),
        }
    }
}

impl AssetsFinder {
    fn visit_expr_call_inner(&mut self, node: &rustpython_ast::ExprCall) -> Result<(), ()> {
        let ident: String = node
            .func
            .as_name_expr()
            .and_then(|o| o.id.parse().ok())
            .or_else(|| {
                node.func
                    .as_attribute_expr()
                    .and_then(|attr| attr.attr.parse().ok())
            })
            .ok_or(())?;

        use AssetKind::*;
        let (access_type, args) = match ident.as_str() {
            "load_s3_file" => (Some(R), vec![(S3Object, Arg::Pos(0, "s3object")), (Resource, Arg::Pos(1, "s3_resource_path"))]),
            "load_s3_file_reader" => (Some(R), vec![(S3Object, Arg::Pos(0, "s3object")), (Resource, Arg::Pos(1, "s3_resource_path"))]),
            "write_s3_file" => (Some(W), vec![(S3Object, Arg::Pos(0, "s3object")), (Resource, Arg::Pos(2, "s3_resource_path"))]),
            "get_resource" => (None, vec![(Resource, Arg::Pos(0, "path"))]),
            "set_resource" => (Some(W), vec![(Resource, Arg::Pos(0, "path"))]),
            "get_boto3_connection_settings" => (None, vec![(Resource, Arg::Pos(0, "s3_resource_path"))]),
            "get_polars_connection_settings" => (None, vec![(Resource, Arg::Pos(0, "s3_resource_path"))]),
            "get_duckdb_connection_settings" => (None, vec![(Resource, Arg::Pos(0, "s3_resource_path"))]),
            _ => return Err(()),
        };

        // Check each possible argument for assets
        for (kind, arg) in args {
            let arg_val = match arg {
                Arg::Pos(i, name) => node.args.get(i).or_else(|| {
                    // Get arg by name
                    node.keywords
                        .iter()
                        .find(|kw| kw.arg.as_deref() == Some(name))
                        .map(|kw| &kw.value)
                }),
            };

            if let Some(Expr::Constant(ExprConstant { value: Constant::Str(value), .. })) = arg_val {
                // For S3 functions, check if the string contains an S3 URI
                if kind == S3Object {
                    if let Some((asset_kind, path)) = parse_asset_syntax(&value) {
                        if asset_kind == AssetKind::S3Object {
                            self.assets
                                .push(ParseAssetsResult { kind, path: path.to_string(), access_type });
                            return Ok(());
                        }
                    }
                } else if kind == Resource {
                    // For resource parameters, accept the value as-is (could be resource path or S3 URI)
                    if let Some((asset_kind, path)) = parse_asset_syntax(&value) {
                        // If it parses as an asset URI, use that
                        self.assets
                            .push(ParseAssetsResult { kind: asset_kind, path: path.to_string(), access_type });
                        return Ok(());
                    } else {
                        // Otherwise, treat it as a resource path
                        self.assets
                            .push(ParseAssetsResult { kind, path: value.to_string(), access_type });
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }
}

enum Arg {
    // Positional arguments in python can also be used by their name
    Pos(usize, &'static str),
}

#[cfg(test)]
mod tests {
    use super::*;
    use windmill_parser::asset_parser::AssetKind;

    #[test]
    fn test_parse_s3_positional_args() {
        let code = r#"
import wmill

def main():
    result = wmill.load_s3_file("s3://my-bucket/my-file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "my-bucket/my-file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_parse_s3_named_args() {
        let code = r#"
import wmill

def main():
    result = wmill.load_s3_file(s3object="s3://my-bucket/my-file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "my-bucket/my-file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_parse_s3_write_named_args() {
        let code = r#"
import wmill

def main():
    result = wmill.write_s3_file(s3object="s3://my-bucket/my-file.txt", data="test")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "my-bucket/my-file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::W));
    }

    #[test]
    fn test_parse_resource_named_args() {
        let code = r#"
import wmill

def main():
    result = wmill.get_resource(path="u/user/my_resource")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::Resource);
        assert_eq!(assets[0].path, "u/user/my_resource");
        assert_eq!(assets[0].access_type, None);
    }

    #[test] 
    fn test_parse_mixed_args() {
        let code = r#"
import wmill

def main():
    # Positional arg
    result1 = wmill.load_s3_file("s3://bucket1/file1.txt")
    # Named arg
    result2 = wmill.write_s3_file(s3object="s3://bucket2/file2.txt", data="test")
    return result1, result2
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 2);
        
        // Assets should be sorted by path
        assert_eq!(assets[0].path, "bucket1/file1.txt");
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
        
        assert_eq!(assets[1].path, "bucket2/file2.txt");
        assert_eq!(assets[1].kind, AssetKind::S3Object);
        assert_eq!(assets[1].access_type, Some(AssetUsageAccessType::W));
    }

    #[test]
    fn test_parse_direct_import() {
        let code = r#"
from wmill import load_s3_file, write_s3_file

def main():
    # Direct function call with named arg
    result = load_s3_file(s3object="s3://my-bucket/my-file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "my-bucket/my-file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_parse_all_s3_functions_with_named_args() {
        let code = r#"
import wmill

def main():
    # All S3-related functions with named arguments
    result1 = wmill.load_s3_file(s3object="s3://bucket1/file1.txt")
    result2 = wmill.load_s3_file_reader(s3object="s3://bucket2/file2.txt")
    result3 = wmill.write_s3_file(s3object="s3://bucket3/file3.txt", data="test")
    return result1, result2, result3
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 3);
        
        // Should find all three S3 objects
        assert_eq!(assets[0].path, "bucket1/file1.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
        assert_eq!(assets[1].path, "bucket2/file2.txt");
        assert_eq!(assets[1].access_type, Some(AssetUsageAccessType::R));
        assert_eq!(assets[2].path, "bucket3/file3.txt");
        assert_eq!(assets[2].access_type, Some(AssetUsageAccessType::W));
    }

    #[test]
    fn test_parse_non_s3_named_args() {
        let code = r#"
import wmill

def main():
    # Test other function with named args
    resource = wmill.get_boto3_connection_settings(s3_resource_path="u/user/my_s3_resource")
    return resource
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::Resource);
        assert_eq!(assets[0].path, "u/user/my_s3_resource");
        assert_eq!(assets[0].access_type, None);
    }

    #[test]
    fn test_parse_wrong_named_arg() {
        // Test case where named arg doesn't match expected parameter name
        let code = r#"
import wmill

def main():
    # Using wrong parameter name - should not be detected
    result = wmill.load_s3_file(wrong_param="s3://bucket/file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        // Should not detect any assets because parameter name doesn't match
        assert_eq!(assets.len(), 0);
    }

    #[test]
    fn test_parse_aliased_import() {
        let code = r#"
import wmill as w

def main():
    result = w.load_s3_file(s3object="s3://my-bucket/file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "my-bucket/file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_parse_multiple_named_params() {
        let code = r#"
import wmill

def main():
    result = wmill.write_s3_file(s3object="s3://bucket/file.txt", data="content", content_type="text/plain")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "bucket/file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::W));
    }

    #[test]
    fn test_parse_complex_real_world_example() {
        // This mimics a more realistic usage pattern
        let code = r#"
import wmill
import pandas as pd

def main(input_data: str):
    # Load some data from S3
    s3_data = wmill.load_s3_file(s3object="s3://data-bucket/input.csv")
    
    # Process the data
    df = pd.read_csv(s3_data)
    processed_data = df.head()
    
    # Save results back to S3
    result = wmill.write_s3_file(
        s3object="s3://data-bucket/output.csv", 
        data=processed_data.to_csv()
    )
    
    # Also get some configuration
    config = wmill.get_resource(path="u/admin/config")
    
    return {"status": "success", "config": config}
"#;
        let assets = parse_assets(code).unwrap();
        assert_eq!(assets.len(), 3);
        
        // Should be sorted: config, input, output
        assert_eq!(assets[0].path, "data-bucket/input.csv");
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
        
        assert_eq!(assets[1].path, "data-bucket/output.csv");
        assert_eq!(assets[1].kind, AssetKind::S3Object);
        assert_eq!(assets[1].access_type, Some(AssetUsageAccessType::W));
        
        assert_eq!(assets[2].path, "u/admin/config");
        assert_eq!(assets[2].kind, AssetKind::Resource);
        assert_eq!(assets[2].access_type, None);
    }

    #[test]
    fn test_parse_s3_resource_path_param() {
        // Test using s3_resource_path parameter instead of s3object
        let code = r#"
import wmill

def main():
    # This pattern was previously failing - now it should work!
    result = wmill.load_s3_file(s3_resource_path="u/admin/s3_config")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        // This should detect the resource, not the S3 object
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::Resource);
        assert_eq!(assets[0].path, "u/admin/s3_config");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_parse_s3_uri_in_resource_path_param() {
        // Test passing an S3 URI to s3_resource_path parameter
        let code = r#"
import wmill

def main():
    # User might pass S3 URI via s3_resource_path parameter
    result = wmill.load_s3_file(s3_resource_path="s3://bucket/file.txt")
    return result
"#;
        let assets = parse_assets(code).unwrap();
        // This should detect as S3Object since the string contains an S3 URI
        assert_eq!(assets.len(), 1);
        assert_eq!(assets[0].kind, AssetKind::S3Object);
        assert_eq!(assets[0].path, "bucket/file.txt");
        assert_eq!(assets[0].access_type, Some(AssetUsageAccessType::R));
    }

    #[test]
    fn test_debug_diegoimbert_case() {
        // Let me try to reproduce the exact issue that @diegoimbert might be seeing
        println!("=== Testing various named argument patterns ===");
        
        let test_cases = vec![
            // Case 1: Standard named arg
            (r#"
import wmill
def main():
    result = wmill.load_s3_file(s3object="s3://bucket/file.txt")
    return result
"#, "Standard named arg"),
            
            // Case 2: Using s3_resource_path instead
            (r#"
import wmill
def main():
    result = wmill.load_s3_file(s3_resource_path="u/admin/s3_config")
    return result
"#, "Using s3_resource_path"),
            
            // Case 3: Mixed positional and named
            (r#"
import wmill
def main():
    result = wmill.load_s3_file("s3://bucket/file.txt", s3_resource_path="u/admin/s3_config")
    return result
"#, "Mixed positional and named"),
        ];

        for (code, description) in test_cases {
            println!("Testing: {}", description);
            let assets = parse_assets(code).unwrap();
            println!("  Found {} assets", assets.len());
            for asset in &assets {
                println!("    {:?}: {} (access: {:?})", asset.kind, asset.path, asset.access_type);
            }
        }
    }
}