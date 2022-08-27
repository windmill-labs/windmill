//! helpers for serde + serde derive attributes

pub fn default_true() -> bool {
    true
}

pub fn is_default<T: Default + std::cmp::PartialEq>(t: &T) -> bool {
    &T::default() == t
}
