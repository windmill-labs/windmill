use windmill_parser::asset_parser::{AssetKind, ParseAssetsResult};

// Parse assets from sql snippets inside e.g sql`SELECT * FROM my_table`
pub fn parse_wmill_sdk_sql_assets(
    kind: AssetKind,
    asset_name: &str,
    schema: Option<&str>,
    sql: &str,
) -> anyhow::Result<Option<Vec<ParseAssetsResult>>> {
    let duckdb_conn_prefix = match kind {
        AssetKind::DataTable => "datatable",
        AssetKind::Ducklake => "ducklake",
        _ => return Err(anyhow::anyhow!("Unsupported asset kind for SQL parsing")),
    };
    let sql_with_attach =
        format!("ATTACH '{duckdb_conn_prefix}://{asset_name}' AS dt; USE dt; {sql}");

    // We use the SQL parser to detect if it's a read or write query
    match crate::parse_assets(&sql_with_attach) {
        Ok(mut sql_assets) => {
            if let Some(schema) = schema {
                for asset in &mut sql_assets.assets {
                    if asset.kind == kind && asset.path.starts_with(asset_name) {
                        asset.path = format!(
                            "{}/{}.{}",
                            asset_name,
                            schema,
                            &asset.path[asset_name.len() + 1..]
                        );
                    }
                }
            }
            return Ok(Some(sql_assets.assets));
        }
        _ => Ok(None),
    }
}
