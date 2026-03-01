pub mod actions;
pub mod models;
pub mod systems;

pub fn routes() -> axum::Router {
    axum::Router::new()
}
