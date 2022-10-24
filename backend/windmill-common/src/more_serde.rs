//! helpers for serde + serde derive attributes

use crate::utils::rd_string;

pub fn default_true() -> bool {
    true
}

pub fn default_id() -> String {
    rd_string(6)
}

pub fn is_default<T: Default + std::cmp::PartialEq>(t: &T) -> bool {
    &T::default() == t
}
