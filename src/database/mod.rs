pub mod repositories;
pub mod models;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn connect() -> Result<PgPool, sqlx::Error> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL no está configurada");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
}
