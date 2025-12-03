use std::collections::HashMap;

use windmill_parser::asset_parser::{
    merge_assets, AssetKind, AssetUsageAccessType, ParseAssetsResult,
};
use AssetUsageAccessType::*;

use nom::{
    branch::alt,
    bytes::complete::{tag, tag_no_case, take_while, take_while1},
    character::complete::{char, multispace0},
    combinator::opt,
    sequence::preceded,
    IResult, Parser,
};

pub fn parse_assets<'a>(input: &str) -> anyhow::Result<Vec<ParseAssetsResult<&str>>> {
    let mut assets = Vec::new();
    let mut remaining = input;
    let mut var_identifiers: HashMap<String, (AssetKind, String)> = HashMap::new();

    while !remaining.trim().is_empty() {
        if let Ok((rest, _)) = parse_comment(remaining) {
            remaining = rest; // skip comment
        }

        if let Ok((rest, (var_identifier, (asset_kind, asset_name)))) =
            parse_var_identifier(remaining)
        {
            var_identifiers.insert(
                var_identifier.to_string(),
                (asset_kind, asset_name.to_string()),
            );
            remaining = rest;
        }

        if let Ok((rest, res)) = parse_asset(remaining, &var_identifiers) {
            assets.push(res);
            remaining = rest;
        } else {
            remaining = &remaining[1..]; // skip 1 char and continue
        }
    }

    Ok(merge_assets(assets))
}

fn parse_var_identifier(input: &str) -> IResult<&str, (&str, (AssetKind, &str))> {
    alt((parse_ducklake_lit, parse_datatable_lit)).parse(input)
}

fn parse_asset<'a, 'b>(
    input: &'a str,
    var_identifiers: &'b HashMap<String, (AssetKind, String)>,
) -> IResult<&'a str, ParseAssetsResult<&'a str>> {
    alt((
        parse_s3_object_read.map(|path| ParseAssetsResult {
            path,
            kind: AssetKind::S3Object,
            access_type: Some(R),
        }),
        parse_s3_object_write.map(|path| ParseAssetsResult {
            path,
            kind: AssetKind::S3Object,
            access_type: Some(W),
        }),
        // Parse ambiguous access_types at the end if we could not find precisely read or copy
        parse_s3_object_lit.map(|path| ParseAssetsResult {
            path,
            kind: AssetKind::S3Object,
            access_type: None,
        }),
        parse_resource_lit.map(|path| ParseAssetsResult {
            path,
            kind: AssetKind::Resource,
            access_type: None,
        }),
    ))
    .parse(input)
}

/// Any expression that reads an s3 asset
fn parse_s3_object_read(input: &str) -> IResult<&str, &str> {
    alt((parse_s3_object_read_fn, parse_s3_object_select_from)).parse(input)
}

/// Any expression that writes to an s3 asset
fn parse_s3_object_write(input: &str) -> IResult<&str, &str> {
    // COPY (...) TO 's3://...'
    let (input, _) = (tag_no_case("TO"), multispace0).parse(input)?;
    let (input, path) = parse_s3_object_lit(input)?;
    Ok((input, path))
}

/// read_parquet('s3://...')
fn parse_s3_object_read_fn(input: &str) -> IResult<&str, &str> {
    let (input, _) = alt((
        tag_no_case("read_parquet"),
        tag_no_case("read_csv"),
        tag_no_case("read_json"),
    ))
    .parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char('(')(input)?;
    let (input, _) = multispace0(input)?;
    let (input, path) = parse_s3_object_lit(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = char(')')(input)?;
    Ok((input, path))
}

/// SELECT ... FROM 's3://...'
fn parse_s3_object_select_from(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag_no_case("FROM").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, path) = parse_s3_object_lit(input)?;
    Ok((input, path))
}
/// 's3://...'
fn parse_s3_object_lit(input: &str) -> IResult<&str, &str> {
    let (input, _) = quote(input)?;
    let (input, _) = tag("s3://").parse(input)?;
    let (input, path) = take_while1(|c| c != '\'' && c != '"')(input)?;
    let (input, _) = quote(input)?;
    Ok((input, path))
}

fn quote(input: &str) -> IResult<&str, char> {
    alt((char('\''), char('\"'))).parse(input)
}

fn parse_resource_lit(input: &str) -> IResult<&str, &str> {
    let (input, _) = quote(input)?;
    let (input, _) = alt((tag("$res:"), tag("res://"))).parse(input)?;
    let (input, path) = take_while1(|c| c != '\'' && c != '"')(input)?;
    let (input, _) = quote(input)?;
    Ok((input, path))
}

fn parse_ducklake_lit(input: &str) -> IResult<&str, (&str, (AssetKind, &str))> {
    let (input, _) = tag_no_case("ATTACH").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = quote(input)?;
    let (input, _) = tag("ducklake").parse(input)?;
    let (input, path) =
        opt(preceded(tag("://"), take_while1(|c| c != '\'' && c != '"'))).parse(input)?;
    let (input, _) = quote(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag_no_case("AS").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, var_identifier) = identifier(input)?;

    let result = (
        path.unwrap_or("main"),
        (AssetKind::Ducklake, var_identifier),
    );
    Ok((input, result))
}

fn parse_datatable_lit(input: &str) -> IResult<&str, (&str, (AssetKind, &str))> {
    let (input, _) = tag_no_case("ATTACH").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = quote(input)?;
    let (input, _) = tag("datatable").parse(input)?;
    let (input, path) =
        opt(preceded(tag("://"), take_while1(|c| c != '\'' && c != '"'))).parse(input)?;
    let (input, _) = quote(input)?;
    let (input, _) = multispace0(input)?;
    let (input, _) = tag_no_case("AS").parse(input)?;
    let (input, _) = multispace0(input)?;
    let (input, var_identifier) = identifier(input)?;

    let result = (
        path.unwrap_or("main"),
        (AssetKind::DataTable, var_identifier),
    );
    Ok((input, result))
}

fn parse_comment(input: &str) -> IResult<&str, &str> {
    let (input, _) = tag("--").parse(input)?;
    let (input, comment) = take_while(|c| c != '\n')(input)?;
    Ok((input, comment))
}

fn identifier(input: &str) -> IResult<&str, &str> {
    take_while1(|c: char| c.is_alphanumeric() || c == '_')(input)
}
