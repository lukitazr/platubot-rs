pub mod models;
pub mod repositories;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL no está configurada");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}
