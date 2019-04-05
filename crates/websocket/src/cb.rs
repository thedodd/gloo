use std::borrow::Cow;

use crate::{
    builder::WebSocketBuilder,
    core::WebSocketCore,
};

/// A WebSocket abstraction providing a standard callbacks interface.
///
/// This is part of the Gloo mid-level API.
pub struct WebSocket(WebSocketCore);

impl WebSocket {
    /// Internal constructor.
    pub(crate) fn new(core: WebSocketCore) -> Self {
        WebSocket(core)
    }

    /// Begin building a new WebSocket which will connect to the given URL.
    ///
    /// This function returns a builder which will allow for assigning callback handlers on the
    /// various events coming from the WebSocket.
    ///
    /// The builder's `build()` method must be called in order to begin using the WebSocket.
    pub fn connect<U: Into<Cow<'static, str>>>(url: U) -> WebSocketBuilder {
        WebSocketBuilder::new(url.into())
    }
}
