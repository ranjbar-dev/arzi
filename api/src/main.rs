use api::{app, AppState};
use sqlx::postgres::PgPoolOptions;

#[tokio::main]
async fn main() {
    // DATABASE_URL connects as the table-owner role and is used ONLY to run
    // migrations. APP_DATABASE_URL connects as the non-superuser,
    // non-BYPASSRLS role (specs/10-target-architecture.md §4; step 1.1's
    // db/init/01-app-role.sh creates it) and is the pool every request uses
    // — this is the role step 1.1's RLS policies actually gate.
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let app_database_url =
        std::env::var("APP_DATABASE_URL").expect("APP_DATABASE_URL must be set");

    // Dev convenience: run pending migrations on boot. Prod uses an explicit
    // `sqlx migrate run` step instead — no magic at startup in production,
    // per specs/10-target-architecture.md §2.1.
    if std::env::var("RUN_MIGRATIONS_ON_BOOT").as_deref() != Ok("0") {
        let migration_pool = PgPoolOptions::new()
            .max_connections(1)
            .connect(&database_url)
            .await
            .expect("failed to connect to database as owner role");
        sqlx::migrate!()
            .run(&migration_pool)
            .await
            .expect("failed to run migrations");
        migration_pool.close().await;
    }

    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&app_database_url)
        .await
        .expect("failed to connect to database as app role");

    let router = app(AppState { pool });

    let addr = std::net::SocketAddr::from(([0, 0, 0, 0], 8080));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    println!("api listening on {addr}");
    axum::serve(listener, router).await.unwrap();
}
