/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use windmill_audit::audit_oss::AuditAuthorable;
use windmill_common::db::{Authable, Authed, AuthedRef};

#[derive(Default, Clone, Debug)]
pub struct OptJobAuthed {
    pub job_id: Option<uuid::Uuid>,
    pub authed: ApiAuthed,
}

#[derive(Clone, Debug, Default, Hash, Eq, PartialEq)]
pub struct ApiAuthed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
    pub username_override: Option<String>,
    pub token_prefix: Option<String>,
}

impl ApiAuthed {
    pub fn to_authed_ref<'e>(&'e self) -> AuthedRef<'e> {
        AuthedRef {
            email: &self.email,
            username: &self.username,
            is_admin: &self.is_admin,
            is_operator: &self.is_operator,
            groups: &self.groups,
            folders: &self.folders,
            scopes: &self.scopes,
            token_prefix: &self.token_prefix,
        }
    }

    pub fn display_username(&self) -> &str {
        self.username_override.as_ref().unwrap_or(&self.username)
    }
}

impl From<ApiAuthed> for Authed {
    fn from(value: ApiAuthed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
            token_prefix: value.token_prefix,
        }
    }
}

impl From<Authed> for ApiAuthed {
    fn from(value: Authed) -> Self {
        Self {
            email: value.email,
            username: value.username,
            is_admin: value.is_admin,
            is_operator: value.is_operator,
            groups: value.groups,
            folders: value.folders,
            scopes: value.scopes,
            username_override: None, // Authed doesn't have this field, so default to None
            token_prefix: value.token_prefix,
        }
    }
}

impl AuditAuthorable for ApiAuthed {
    fn username(&self) -> &str {
        self.username.as_str()
    }
    fn email(&self) -> &str {
        self.email.as_str()
    }
    fn username_override(&self) -> Option<&str> {
        self.username_override.as_deref()
    }
    fn token_prefix(&self) -> Option<&str> {
        self.token_prefix.as_deref()
    }
}

impl Authable for ApiAuthed {
    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[std::string::String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }
}
