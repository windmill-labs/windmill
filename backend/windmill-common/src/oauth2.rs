/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

use hmac::Hmac;
use sha2::Sha256;

pub type HmacSha256 = Hmac<Sha256>;

pub const WORKSPACE_SLACK_BOT_TOKEN_PATH: &str = "f/slack_bot/bot_token";
