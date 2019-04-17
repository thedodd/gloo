use std::borrow::Cow;

use crate::{
    builder::WebSocketBuilder,
    common::ReadyState,
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

    //////////////////////////////////////////////////////////////////////////
    // Primary Interface /////////////////////////////////////////////////////

    // TODO:
    // - close method with code & reason options.
    // - send method taking a WsMessage.

    /// The number of bytes of data that have been queued but not yet transmitted to the network.
    ///
    /// **NOTE:** that this is the number of bytes buffered by the underlying platform WebSocket
    /// implementation. It does not reflect any buffering performed by this Gloo WebSocket type.
    pub fn buffered_amount(&self) -> u32 {
        self.0.ws.borrow().buffered_amount()
    }

    /// The extensions selected by the server as negotiated during the connection.
    pub fn extensions(&self) -> String {
        self.0.ws.borrow().extensions()
    }

    /// The name of the subprotocol the server selected during the connection.
    ///
    /// This will be one of the strings specified in the protocols parameter when
    /// building this WebSocket instance.
    pub fn protocol(&self) -> String {
        self.0.ws.borrow().protocol()
    }

    /// The current state of the WebSocket connection.
    pub fn ready_state(&self) -> ReadyState {
        ReadyState::from(self.0.ws.borrow().ready_state())
    }

    /// The absolute URL which this WebSocket instance is connected to.
    pub fn url(&self) -> String {
        self.0.ws.borrow().url()
    }
}
