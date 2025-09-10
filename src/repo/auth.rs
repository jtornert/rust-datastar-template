use surrealdb::opt::auth::Record;
use uuid::Uuid;

use crate::{
    CONFIG,
    repo::{
        Db,
        error::{Error, Result},
    },
};

const ACCESS_NAME: &str = "users";

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct Credentials {
    pub(in crate::repo::auth) username: Option<String>,
    pub(in crate::repo::auth) password: Option<String>,
    token_str: Option<String>,
}

impl Credentials {
    #[cfg(test)]
    pub fn with_username_password(username: String, password: String) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
            token_str: None,
        }
    }

    pub fn with_token(token: String) -> Self {
        Self {
            username: None,
            password: None,
            token_str: token.into(),
        }
    }

    pub fn verify(&self) -> Result<()> {
        if let (Some(username), Some(password)) = (self.username.as_ref(), self.password.as_ref()) {
            let errors = [verify_username(username), verify_password(password)].concat();

            if !errors.is_empty() {
                return Err(Error::InvalidCredentials(username.clone(), errors));
            }
        }

        Ok(())
    }
}

fn verify_username(username: &str) -> Vec<&'static str> {
    let mut errors = Vec::new();

    if username.len() < 2 || username.len() > 32 {
        errors.push("username_invalid_length");
    }

    if username.matches(char::is_alphanumeric).next().is_none()
        || username
            .matches(|c: char| !c.is_alphanumeric() && c != '_')
            .next()
            .is_some()
        || username.matches(char::is_whitespace).next().is_some()
    {
        errors.push("username_invalid_characters");
    }

    if !username.chars().next().is_some_and(char::is_alphanumeric)
        || !username
            .chars()
            .next_back()
            .is_some_and(char::is_alphanumeric)
    {
        errors.push("username_invalid_format");
    }

    errors
}

fn verify_password(password: &str) -> Vec<&'static str> {
    let mut errors = Vec::new();

    if password.len() < 8 {
        errors.push("password_invalid_length");
    }

    if password.matches(char::is_alphanumeric).next().is_none()
        || password
            .matches(|c: char| !c.is_alphanumeric())
            .next()
            .is_none()
        || password.matches(char::is_whitespace).next().is_some()
    {
        errors.push("password_invalid_characters");
    }

    errors
}

pub async fn signup(db: &Db<'_>, credentials: Credentials) -> Result<()> {
    credentials.verify()?;

    #[cfg(any(debug_assertions, test))]
    db.invalidate().await.map_err(|e| {
        let uuid = Uuid::new_v4();
        crate::log_line!(uuid, e);
        Error::ServiceUnavailable(uuid)
    })?;
    #[cfg(not(debug_assertions))]
    {
        use crate::CONFIG;
        use surrealdb::opt::auth::Root;

        db.signin(Root {
            username: &CONFIG.db_username,
            password: &CONFIG.db_password,
        })
        .await
        .map_err(|e| {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        })?;
        db.use_ns(&CONFIG.db_namespace)
            .use_db(&CONFIG.db_database)
            .await
            .map_err(|e| {
                let uuid = Uuid::new_v4();
                crate::log_line!(uuid, e);
                Error::ServiceUnavailable(uuid)
            })?;
    }
    match db
        .query("fn::auth::signup($username, $password)")
        .bind(("username", credentials.username))
        .bind(("password", credentials.password))
        .await
        .map_err(|e| {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        })?
        .check()
    {
        Err(surrealdb::Error::Db(surrealdb::error::Db::IndexExists { index, value, .. }))
            if index == "user_name" =>
        {
            Err(Error::UsernameTaken(value))
        }
        Err(e) => {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Err(Error::ServiceUnavailable(uuid))
        }
        Ok(_) => Ok(()),
    }
}

pub async fn login(db: &Db<'_>, credentials: Credentials) -> Result<String> {
    db.signin(Record {
        namespace: &CONFIG.db_namespace,
        database: &CONFIG.db_database,
        access: ACCESS_NAME,
        params: credentials,
    })
    .await
    .map_err(|e| {
        if matches!(e, surrealdb::Error::Db(surrealdb::error::Db::NoRecordFound)) {
            Error::CredentialsInvalid
        } else {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        }
    })?;

    let mut response = db
        .query("fn::auth::create_session($auth)")
        .await
        .map_err(|e| {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        })?;
    let token: Option<String> = response.take("token").map_err(|e| {
        let uuid = Uuid::new_v4();
        crate::log_line!(uuid, e);
        Error::ServiceUnavailable(uuid)
    })?;

    token.ok_or(Error::CredentialsInvalid)
}

pub async fn authenticate(db: &Db<'_>, token: &str) -> Result<()> {
    db.signin(Record {
        namespace: &CONFIG.db_namespace,
        database: &CONFIG.db_database,
        access: ACCESS_NAME,
        params: Credentials::with_token(token.into()),
    })
    .await
    .map_err(|e| {
        if matches!(e, surrealdb::Error::Db(surrealdb::error::Db::NoRecordFound)) {
            tracing::error!(?e);
            Error::CredentialsInvalid
        } else {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        }
    })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        _test::setup_test,
        repo::auth::{verify_password, verify_username},
    };

    #[test]
    fn verify_usernames() {
        setup_test();

        assert_ne!(verify_username("").len(), 0);
        assert_ne!(verify_username("a").len(), 0);
        assert_ne!(verify_username(&"a".repeat(33)).len(), 0);
        assert_ne!(verify_username("__").len(), 0);
        assert_ne!(verify_username("a__").len(), 0);
        assert_ne!(verify_username("__c").len(), 0);
        assert_ne!(verify_username("a_").len(), 0);
        assert_ne!(verify_username("_b").len(), 0);
        assert_eq!(verify_username("ab").len(), 0);
        assert_eq!(verify_username("a_b").len(), 0);
        assert_eq!(verify_username("a_b_c").len(), 0);
    }

    #[test]
    fn verify_passwords() {
        setup_test();

        assert_ne!(verify_password("").len(), 0);
        assert_ne!(verify_password("        ").len(), 0);
        assert_ne!(verify_password("abcdefgh").len(), 0);
        assert_ne!(verify_password("ABCDEFGH").len(), 0);
        assert_ne!(verify_password("12345678").len(), 0);
        assert_ne!(verify_password("++++++++").len(), 0);
        assert_eq!(verify_password("ABcd123+").len(), 0);
    }
}
