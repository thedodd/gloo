use std::borrow::Cow;

use wasm_bindgen::JsValue;

use crate::{
    builder::WebSocketBuilder,
    common::{ReadyState, WsMessage},
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

    /// The number of bytes of data that have been queued but not yet transmitted to the network.
    ///
    /// **NOTE:** that this is the number of bytes buffered by the underlying platform WebSocket
    /// implementation. It does not reflect any buffering performed by this Gloo WebSocket type.
    pub fn buffered_amount(&self) -> u32 {
        self.0.ws.borrow().buffered_amount()
    }

    /// Close this WebSocket connection.
    ///
    /// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/close)
    ///
    /// After this method has been called, the instance is no longer good for much. It would be
    /// logical to drop the instance after calling this method. This type also has a custom
    /// `Drop` implementation which will call this method with default values when the instance
    /// is dropped. Therefore, this method is primarily intended for use when a custom code or
    /// reason needs to be used when closing the connection.
    ///
    /// Any successive calls to this method will be a no-op.
    ///
    /// #### code
    /// If a code is provided, the connection will be closed with the given code. If this
    /// parameter is not specified, a default value of 1005 is assumed. An error will
    /// be returned if an invalid code is provided.
    /// [MDN list of valid codes](https://developer.mozilla.org/en-US/docs/Web/API/CloseEvent#Status_codes).
    ///
    /// #### reason
    /// If a reason is supplied, it will be used as the explanation for why the connection was
    /// closed. The string must be no longer than `123` bytes, else an exception will be thrown
    /// from the underlying platform.
    pub fn close(&mut self, code: Option<u16>, reason: Option<String>) -> Result<(), JsValue> {
        self.0.close(code.unwrap_or(1005u16), reason)
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

    /// Send the given message over the WebSocket.
    ///
    /// The message will be send as either a text frame or a binary frame based on which variant
    /// of `WsMessage` is being used.
    pub fn send(&self, msg: WsMessage) -> Result<(), JsValue> {
        match msg {
            WsMessage::Binary(mut payload) => self.0.ws.borrow().send_with_u8_array(payload.as_mut_slice()),
            WsMessage::Text(payload) => self.0.ws.borrow().send_with_str(payload.as_ref()),
        }
    }

    /// The absolute URL which this WebSocket instance is connected to.
    pub fn url(&self) -> String {
        self.0.ws.borrow().url()
    }
}

impl Drop for WebSocket {
    fn drop(&mut self) {
        // Ensure the underlying WebSocket instance is dropped.
        let _ = self.close(None, None);
    }
}
