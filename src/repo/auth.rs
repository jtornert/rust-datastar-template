use surrealdb::opt::auth::Record;

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
}

pub async fn signup(db: &Db<'_>, credentials: Credentials) -> Result<()> {
    #[cfg(any(debug_assertions, test))]
    db.invalidate().await.unwrap();
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
            tracing::error!(?e);
            Error::Surreal
        })?;
        db.use_ns(&CONFIG.db_namespace)
            .use_db(&CONFIG.db_database)
            .await
            .map_err(|e| {
                tracing::error!(?e);
                Error::Surreal
            })?;
    }
    match db
        .query("fn::auth::signup($username, $password)")
        .bind(("username", credentials.username))
        .bind(("password", credentials.password))
        .await
        .map_err(|e| {
            tracing::error!(?e);
            Error::Surreal
        })?
        .check()
    {
        Err(surrealdb::Error::Db(surrealdb::error::Db::IndexExists { index, value, .. }))
            if index == "user_name" =>
        {
            Err(Error::UsernameTaken(value))
        }
        Err(surrealdb::Error::Db(surrealdb::error::Db::FieldValue { value, field, .. })) => {
            if field.to_string() == "name" {
                return Err(Error::InvalidUsername);
            }
            tracing::error!("{} = {}", field, value);
            Err(Error::Surreal)
        }
        Err(e) => {
            tracing::error!(?e);
            Err(Error::Surreal)
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
            tracing::error!(?e);
            Error::Surreal
        }
    })?;

    let mut response = db
        .query("fn::auth::create_session($auth)")
        .await
        .map_err(|e| {
            tracing::error!(?e);
            Error::Surreal
        })?;
    let token: Option<String> = response.take("token").map_err(|e| {
        tracing::error!(?e);
        Error::Surreal
    })?;

    token.ok_or(Error::Surreal)
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
        tracing::error!(?e);
        Error::CredentialsInvalid
    })?;

    Ok(())
}
