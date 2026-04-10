pub mod api;
pub mod domain;
pub mod error;
pub mod infrastructure;
pub mod usecases;

#[derive(Clone)]
pub struct AppState {
    pub db_pool: sqlx::PgPool,
}
