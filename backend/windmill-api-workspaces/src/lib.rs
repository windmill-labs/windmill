pub mod data_metrics;
pub mod datatable_acl;
pub mod datatable_acl_oss;
pub mod datatable_migrations;
pub mod datatable_permissions;
pub mod datatable_permissions_oss;
pub mod deployment_requests;
pub mod workspaces;
pub mod workspaces_extra;
pub mod workspaces_oss;

// Community builds carry `private`, so the data table planners take the
// enterprise feature too: what they plan is enterprise-only.
#[cfg(all(feature = "private", feature = "enterprise"))]
pub mod datatable_acl_ee;
#[cfg(all(feature = "private", feature = "enterprise"))]
pub mod datatable_permissions_ee;
#[cfg(feature = "private")]
pub mod workspaces_ee;
