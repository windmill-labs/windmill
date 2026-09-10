pub mod datatable_acl;
pub mod datatable_acl_oss;
pub mod datatable_migrations;
pub mod datatable_permissions;
pub mod deployment_requests;
pub mod data_metrics;
pub mod workspaces;
pub mod workspaces_extra;
pub mod workspaces_oss;

#[cfg(feature = "private")]
pub mod workspaces_ee;

#[cfg(all(feature = "private", feature = "enterprise"))]
pub mod datatable_acl_ee;
