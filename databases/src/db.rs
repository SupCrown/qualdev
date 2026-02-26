use sqlx::{PgPool, postgres::PgPoolOptions};

pub async fn connect_db() -> PgPool {
    let database_url =
        std::env::var("DATABASE_URL").expect("DATABASE_URL non défini");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Connexion à la BDD échouée")
}
