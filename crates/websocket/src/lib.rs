//! The Gloo project's mid-level & high-level WebSocket APIs.
//!
//! For a callbacks-based API, use the `cb::WebSocket` type.
//!
//! For a futures-based API, use the `fut::WebSocket` type.
//!
//! The `common` module provides types and functionality common to both API types.

pub mod cb;
pub mod common;
pub mod fut;

pub use crate::{
    common::{
        ReadyState,
        ReconnectConfig,
        WsEvent,
        WsMessage,
    },
};
