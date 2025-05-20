/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

mod jobs;
pub mod jobs_ee;
pub mod jobs_oss;
pub mod schedule;

#[cfg(feature = "private")]
pub use jobs_ee::*;
#[cfg(not(feature = "private"))]
pub use jobs_oss::*;

pub use jobs::*;
pub mod flow_status;
pub mod tags;
