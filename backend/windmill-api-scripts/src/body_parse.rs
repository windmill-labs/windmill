/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2026
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

//! One parse of a script body, for its assets and for the workspace items it references.

use windmill_common::scripts::ScriptLang;
use windmill_parser::{
    asset_parser::ParseAssetsOutput,
    item_reference::{scan_sdk_calls, ItemKind, ItemReference, ReferenceAccess},
};

pub use windmill_parser::{asset_parser, item_reference};
pub use windmill_parser_ts_asset::parse_app_file;

pub struct ParsedBody {
    /// The body's assets and references. When no AST parser read it, only the references the
    /// lexical scan found.
    pub output: ParseAssetsOutput,
    /// Whether an AST parser read the body.
    pub parsed: bool,
    /// Why the AST parser failed, when it did.
    pub error: Option<anyhow::Error>,
}

impl ParsedBody {
    fn scanned(references: Vec<ItemReference>, error: Option<anyhow::Error>) -> Self {
        ParsedBody {
            output: ParseAssetsOutput { references, ..Default::default() },
            parsed: false,
            error,
        }
    }
}

pub fn parse_body(lang: &ScriptLang, content: &str) -> ParsedBody {
    let parsed = match lang {
        ScriptLang::DuckDb => windmill_parser_sql_asset::parse_assets(content),
        ScriptLang::Bun | ScriptLang::Bunnative | ScriptLang::Deno | ScriptLang::Nativets => {
            windmill_parser_ts_asset::parse_assets(content)
        }
        #[cfg(feature = "python")]
        ScriptLang::Python3 => windmill_parser_py_asset::parse_assets(content),
        ScriptLang::Ansible => windmill_parser_yaml::parse_assets(content),
        // Their executors read the resource on the `-- database` line with the job's identity.
        ScriptLang::Postgresql
        | ScriptLang::Mysql
        | ScriptLang::Bigquery
        | ScriptLang::Snowflake
        | ScriptLang::Mssql
        | ScriptLang::OracleDB => {
            let database = windmill_parser_sql::parse_db_resource(content)
                .and_then(|p| ItemReference::of(ItemKind::Resource, ReferenceAccess::Read, &p));
            return ParsedBody::scanned(database.into_iter().collect(), None);
        }
        _ => return ParsedBody::scanned(scan_sdk_calls(content), None),
    };
    match parsed {
        Ok(output) => ParsedBody { output, parsed: true, error: None },
        Err(e) => ParsedBody::scanned(scan_sdk_calls(content), Some(e)),
    }
}
