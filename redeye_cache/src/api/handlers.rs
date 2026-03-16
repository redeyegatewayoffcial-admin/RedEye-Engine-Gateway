use axum::{
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde_json::json;
use std::sync::Arc;

use crate::domain::models::{CacheLookupRequest, CacheStoreRequest};
use crate::usecases::semantic_search::SemanticSearchUseCase;

#[derive(Clone)]
pub struct ApiState {
    pub search_use_case: Arc<SemanticSearchUseCase>,
}

pub async fn lookup_handler(
    State(state): State<ApiState>,
    Json(req): Json<CacheLookupRequest>,
) -> impl IntoResponse {
    match state.search_use_case.check_cache(&req).await {
        Ok(Some(cached_run)) => (StatusCode::OK, Json(json!({"hit": true, "data": cached_run}))),
        Ok(None) => (StatusCode::OK, Json(json!({"hit": false}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    }
}

pub async fn store_handler(
    State(state): State<ApiState>,
    Json(req): Json<CacheStoreRequest>,
) -> impl IntoResponse {
    match state.search_use_case.store_response(&req).await {
        Ok(_) => (StatusCode::CREATED, Json(json!({"stored": true}))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e}))),
    }
}
