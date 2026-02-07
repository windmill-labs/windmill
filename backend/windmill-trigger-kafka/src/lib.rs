/*
 * Author: Windmill Labs, Inc
 * Copyright: Windmill Labs, Inc 2024
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

#[cfg(feature = "private")]
mod handler_ee;
pub mod handler_oss;

#[cfg(feature = "private")]
mod listener_ee;
pub mod listener_oss;

#[cfg(feature = "private")]
mod mod_ee;
#[cfg(feature = "private")]
pub use mod_ee::*;

#[derive(Copy, Clone)]
pub struct KafkaTrigger;
