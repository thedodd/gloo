//! The Gloo project's mid-level & high-level WebSocket APIs.
//!
//! For a callbacks-based API, use the `cb::WebSocket` type.
//!
//! For a futures-based API, use the `fut::WebSocket` type.
//!
//! The `common` module provides types and functionality common to both API types.

mod builder;
pub mod cb;
pub mod common;
mod core;
pub mod fut;
