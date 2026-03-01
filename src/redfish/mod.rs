pub mod actions;
pub mod systems;

pub fn routes() -> axum::Router {
    axum::Router::new()
}
