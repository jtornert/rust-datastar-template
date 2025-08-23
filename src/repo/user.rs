use crate::{
    models::user::User,
    repo::{Db, DbRecord, Error, Result},
};

pub async fn me(db: &Db<'_>) -> Result<DbRecord<User>> {
    let mut response = match db
        .query("fn::user::get($auth)")
        .await
        .map_err(|e| {
            tracing::error!(?e);
            Error::Surreal
        })?
        .check()
    {
        Err(e) => {
            tracing::error!(?e);
            return Err(Error::Surreal);
        }
        Ok(response) => response,
    };
    let user: Option<DbRecord<User>> = response.take(0).map_err(|e| {
        tracing::error!(?e);
        Error::Surreal
    })?;

    user.ok_or(Error::Surreal)
}
