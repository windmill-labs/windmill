/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub mod triggers;
#[cfg(feature = "native_trigger")]
pub mod native_triggers;

pub mod args_ext;
pub mod capture_ext;
pub mod flow_ext;
pub mod jobs_ext;
pub mod resource_ext;
pub mod script_ext;
pub mod user_ext;
