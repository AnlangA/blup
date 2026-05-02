use axum::routing::{delete, get, post};
use axum::Router;
use tower_http::cors::CorsLayer;

use super::handlers;
use crate::AppState;

pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(handlers::health))
        .route("/api/session", post(handlers::create_session))
        .route("/api/sessions", get(handlers::list_sessions))
        .route("/api/session/:id", get(handlers::get_session_status))
        .route("/api/session/:id", delete(handlers::delete_session))
        .route("/api/session/:id/goal", post(handlers::submit_goal))
        .route(
            "/api/session/:id/goal/stream",
            post(handlers::submit_goal_stream),
        )
        .route(
            "/api/session/:id/profile/answer",
            post(handlers::submit_profile_answer),
        )
        .route("/api/session/:id/curriculum", get(handlers::get_curriculum))
        .route(
            "/api/session/:id/chapter/:ch_id",
            get(handlers::start_chapter),
        )
        .route(
            "/api/session/:id/chapter/:ch_id/stream",
            get(handlers::start_chapter_stream),
        )
        .route(
            "/api/session/:id/chapter/:ch_id/ask",
            post(handlers::ask_question),
        )
        .route(
            "/api/session/:id/chapter/:ch_id/complete",
            post(handlers::complete_chapter),
        )
        .route(
            "/api/session/:id/chapter/:ch_id/exercise/:ex_id/submit",
            post(handlers::submit_exercise),
        )
        .route("/api/session/:id/progress", get(handlers::get_all_progress))
        .route(
            "/api/session/:id/messages",
            get(handlers::get_messages_paginated),
        )
        .layer(CorsLayer::permissive())
        .with_state(state)
}
