//! OSS fallback for enterprise data-table permissions (implementation in
//! windmill-ee-private, see `datatable_permissions_ee`). The public build has no
//! RLS/role provisioning: reconciling reports that the enterprise edition is
//! required, and query-time resolution always falls through to the default
//! connection (the legacy behavior, where every workspace member shares the
//! owner role and has full access).

use crate::{
    error::{Error, Result},
    workspaces::{DataTable, DatatableAccessDecision},
    DB,
};

/// Reconcile the target database (roles, grants, RLS policies) to match the
/// data table's stored permission config. Enterprise-only.
pub async fn reconcile_datatable_permissions(
    _db: &DB,
    _w_id: &str,
    _datatable_name: &str,
    _datatable: &DataTable,
) -> Result<Vec<String>> {
    Err(Error::internal_err(
        "Data table permissions require the enterprise edition".to_string(),
    ))
}

/// Decide which Postgres identity a query against `datatable` should run under
/// for the acting user. The public build never overrides the connection.
pub async fn resolve_datatable_access(
    _db: &DB,
    _w_id: &str,
    _datatable_name: &str,
    _datatable: &DataTable,
    _base_creds: &serde_json::Value,
    _acting_email: &str,
    _is_workspace_admin: bool,
) -> Result<DatatableAccessDecision> {
    Ok(DatatableAccessDecision::Default)
}
