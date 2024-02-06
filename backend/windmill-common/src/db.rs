use sqlx::{Pool, Postgres, Transaction};

pub type DB = Pool<Postgres>;

#[derive(Clone, Debug)]
pub struct Authed {
    pub email: String,
    pub username: String,
    pub is_admin: bool,
    pub is_operator: bool,
    pub groups: Vec<String>,
    // (folder name, can write, is owner)
    pub folders: Vec<(String, bool, bool)>,
    pub scopes: Option<Vec<String>>,
}

#[derive(Clone)]
pub struct UserDB {
    db: DB,
}

pub trait Authable {
    fn email(&self) -> &str;
    fn username(&self) -> &str;
    fn is_admin(&self) -> bool;
    fn is_operator(&self) -> bool;
    fn groups(&self) -> &[String];
    fn folders(&self) -> &[(String, bool, bool)];
    fn scopes(&self) -> Option<&[String]>;
}

impl Authable for Authed {
    fn is_admin(&self) -> bool {
        self.is_admin
    }

    fn is_operator(&self) -> bool {
        self.is_operator
    }

    fn groups(&self) -> &[String] {
        &self.groups
    }

    fn folders(&self) -> &[(String, bool, bool)] {
        &self.folders
    }

    fn scopes(&self) -> Option<&[std::string::String]> {
        self.scopes.as_ref().map(|x| x.as_slice())
    }

    fn email(&self) -> &str {
        &self.email
    }

    fn username(&self) -> &str {
        &self.username
    }
}

impl UserDB {
    pub fn new(db: DB) -> Self {
        Self { db }
    }

    pub async fn begin<T>(self, authed: &T) -> Result<Transaction<'static, Postgres>, sqlx::Error>
    where
        T: Authable,
    {
        let user = if authed.is_admin() {
            "windmill_admin"
        } else {
            "windmill_user"
        };

        let (folders_write, folders_read): &(Vec<_>, Vec<_>) =
            &authed.folders().into_iter().partition(|x| x.1);

        let mut folders_read = folders_read.clone();
        folders_read.extend(folders_write.clone());

        let mut tx = self.db.begin().await?;

        sqlx::query(&format!("SET LOCAL ROLE {}", user))
            .execute(&mut *tx)
            .await?;

        sqlx::query!(
            "SELECT set_config('session.user', $1, true)",
            authed.username()
        )
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.groups', $1, true)",
            &authed.groups().join(",")
        )
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.pgroups', $1, true)",
            &authed
                .groups()
                .iter()
                .map(|x| format!("g/{}", x))
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.folders_read', $1, true)",
            folders_read
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut *tx)
        .await?;

        sqlx::query!(
            "SELECT set_config('session.folders_write', $1, true)",
            folders_write
                .iter()
                .map(|x| x.0.clone())
                .collect::<Vec<_>>()
                .join(",")
        )
        .fetch_optional(&mut *tx)
        .await?;

        Ok(tx)
    }
}
