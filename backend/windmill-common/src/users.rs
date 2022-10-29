/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub fn owner_to_token_owner(user: &str, is_group: bool) -> String {
    let prefix = if is_group { 'g' } else { 'u' };
    format!("{}/{}", prefix, user)
}
