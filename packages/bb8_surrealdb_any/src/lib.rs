use bb8::ManageConnection;
use surrealdb::{
    Surreal,
    engine::any::{Any, IntoEndpoint},
};

pub struct ConnectionManager<T: IntoEndpoint + Clone + Send + Sync + 'static> {
    address: T,
}

impl<T: IntoEndpoint + Clone + Send + Sync + 'static> ConnectionManager<T> {
    pub fn new(address: T) -> Self {
        Self { address }
    }
}

impl<T: IntoEndpoint + Clone + Send + Sync + 'static> ManageConnection for ConnectionManager<T> {
    type Connection = Surreal<Any>;
    type Error = surrealdb::Error;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        let conn = surrealdb::engine::any::connect(self.address.clone()).await?;

        conn.use_ns(&std::env::var("DB_NAMESPACE").unwrap_or_else(|_| "test".into()))
            .use_db(&std::env::var("DB_DATABASE").unwrap_or_else(|_| "test".into()))
            .await?;

        Ok(conn)
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        conn.health().await
    }

    fn has_broken(&self, _: &mut Self::Connection) -> bool {
        false
    }
}
