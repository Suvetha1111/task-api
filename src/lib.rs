//! Task-management API.
//!
//! Layering (request flows top to bottom):
//!
//! * `routes`       – map URLs to handlers
//! * `controllers`  – parse HTTP, read the authenticated user, call a manager
//! * `managers`     – orchestrate one use case
//! * `services`     – business rules (role checks, 2FA, cache hit/miss)
//! * `repositories` – SQL only
//! * `containers`   – build `Arc<dyn Trait>` graphs stored on [`state::AppState`]
//!
//! The library exposes [`app::build_state`] and [`app::configure`] so the binary
//! and the integration tests boot exactly the same application.

pub mod app;
pub mod config;
pub mod containers;
pub mod controllers;
pub mod error;
pub mod managers;
pub mod middleware;
pub mod models;
pub mod repositories;
pub mod routes;
pub mod services;
pub mod state;
pub mod swagger;
