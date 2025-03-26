/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use phf::phf_map;
type PyMap = phf::Map<&'static str, &'static str>;

/// In some cases inferring requirement from import is not possible.
/// That's why we need to map import to requirement ident.
/// These two maps allows us to do so.

/// import x.y.z
///        ^^^^^ replaces entire x.y.z import with [?]
///
pub static FULL_IMPORTS_MAP: PyMap = phf_map! {
    //              import => requirement
    "google.cloud.webrisk" => "google-cloud-webrisk",
    // Add new entry here ^
};

/// import x.y.z
///        ^ replaces x with [?]
///
/// Additional rules:
/// 1. in x "_" are replaced with "-"
/// 2. If full imports had a hit, this one will not be called
pub static SHORT_IMPORTS_MAP: PyMap = phf_map! {
    //  import => requirement
    "psycopg2" => "psycopg2-binary",
    "psycopg" => "psycopg[binary, pool]",
    "yaml" => "pyyaml",
    "git" => "GitPython",
    "shopify" => "ShopifyAPI",
    "seleniumwire" => "selenium-wire",
    "openbb-terminal" => "openbb[all]",
    "riskfolio" => "riskfolio-lib",
    "smb" => "pysmb",
    "PIL" => "Pillow",
    "googleapiclient" => "google-api-python-client",
    "googlecloudbigquery" => "google-cloud-bigquery",
    "dateutil" => "python-dateutil",
    "mailparser" => "mail-parser",
    "mailparser-reply" => "mail-parser-reply",
    "gitlab" => "python-gitlab",
    "smbclient" => "smbprotocol",
    "playhouse" => "peewee",
    "dns" => "dnspython",
    "msoffcrypto" => "msoffcrypto-tool",
    "tabula" => "tabula-py",
    "shapefile" => "pyshp",
    "sklearn" => "scikit-learn",
    "umap" => "umap-learn",
    "cv2" => "opencv-python",
    "atlassian" => "atlassian-python-api",
    "mysql" => "mysql-connector-python",
    "tenable" => "pytenable",
    "ns1" => "ns1-python",
    "pymsql" => "PyMySQL",
    "haystack" => "haystack-ai",
    "github" => "PyGithub",
    "ldap" => "python-ldap",
    "opensearchpy" => "opensearch-py",
    "lokalise" => "python-lokalise-api",
    "msgraph" => "msgraph-sdk",
    "pythonjsonlogger" => "python-json-logger",
    "socks" => "PySocks",
    "taiga" => "python-taiga",
    "docx" => "python-docx",
    // Add new entry here ^
};
