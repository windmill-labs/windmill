/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use std::sync::atomic::AtomicBool;

use hmac::Hmac;
use serde::Serialize;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

pub const WORKSPACE_SLACK_BOT_TOKEN_PATH: &str = "f/slack_bot/bot_token";

pub const GLOBAL_SLACK_BOT_TOKEN_PATH: &str = "f/slack_bot/global_bot_token";

pub const GLOBAL_TEAMS_BOT_TOKEN_PATH: &str = "f/teams_bot/global_bot_token";

pub const GLOBAL_TEAMS_API_TOKEN_PATH: &str = "f/teams_bot/global_api_token";
lazy_static::lazy_static! {

    pub static ref REQUIRE_PREEXISTING_USER_FOR_OAUTH: AtomicBool = AtomicBool::new(std::env::var("REQUIRE_PREEXISTING_USER_FOR_OAUTH")
        .ok()
        .and_then(|x| x.parse::<bool>().ok())
        .unwrap_or(false));

}

#[derive(Serialize)]
#[serde(tag = "type")]
pub enum InstanceEvent {
    UserSignupOAuth { email: String },
    UserAdded { email: String },
    // UserDeleted { email: String },
    // UserDeletedWorkspace { workspace: String, email: String },
    UserAddedWorkspace { workspace: String, email: String },
    UserInvitedWorkspace { workspace: String, email: String },
    UserJoinedWorkspace { workspace: String, email: String, username: String },
}
