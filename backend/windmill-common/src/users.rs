pub fn owner_to_token_owner(user: &str, is_group: bool) -> String {
    let prefix = if is_group { 'g' } else { 'u' };
    format!("{}/{}", prefix, user)
}
