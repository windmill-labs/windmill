/*
 * Author: Ruben Fiszel
 * Copyright: Windmill Labs, Inc 2022
 * This file and its contents are licensed under the AGPLv3 License.
 * Please see the included NOTICE for copyright information and
 * LICENSE-AGPL for a copy of the license.
 */

pub fn username_to_permissioned_as(user: &str) -> String {
    if user.contains('@') {
        user.to_string()
    } else {
        format!("u/{}", user)
    }
}
