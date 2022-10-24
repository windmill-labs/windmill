pub fn owner_to_token_owner(user: &str, is_group: bool) -> String {
    let prefix = if is_group { 'g' } else { 'u' };
    format!("{}/{}", prefix, user)
}

#[derive(Clone, Debug)]
pub struct Authed {
    pub email: Option<String>,
    pub username: String,
    pub is_admin: bool,
    pub groups: Vec<String>,
}
