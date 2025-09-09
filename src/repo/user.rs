use uuid::Uuid;

use crate::{
    models::user::User,
    repo::{Db, DbRecord, Error, Result},
};

pub async fn me(db: &Db<'_>) -> Result<DbRecord<User>> {
    let mut response = match db
        .query("fn::user::get($auth)")
        .await
        .map_err(|e| {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            Error::ServiceUnavailable(uuid)
        })?
        .check()
    {
        Err(e) => {
            let uuid = Uuid::new_v4();
            crate::log_line!(uuid, e);
            return Err(Error::ServiceUnavailable(uuid));
        }
        Ok(response) => response,
    };
    let user: Option<DbRecord<User>> = response.take(0).map_err(|e| {
        let uuid = Uuid::new_v4();
        crate::log_line!(uuid, e);
        Error::ServiceUnavailable(uuid)
    })?;

    user.ok_or(Error::NotFound)
}
