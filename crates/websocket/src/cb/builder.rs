use std::{
    borrow::Cow,
    cell::{RefCell},
    rc::Rc,
};

use wasm_bindgen::{
    JsValue,
    closure::Closure,
};
use web_sys::{self, Event};

use crate::{
    cb::{
        WebSocket,
        core::WebSocketCore,
    },
    common::{
        ReconnectConfig,
        WsMessage,
    },
};

/// A type used for building a new WebSocket instance.
pub struct WebSocketBuilder {
    /// User supplied URL for the WebSocket connection.
    pub(crate) url: Rc<Cow<'static, str>>,

    /// User supplied subprotocols for the WebSocket to use.
    pub(crate) protocols: Option<Rc<Vec<Cow<'static, str>>>>,

    /// User supplied `message` handler.
    pub(crate) onmessage: Option<Rc<RefCell<dyn FnMut(WsMessage)>>>,

    /// User supplied `open` handler.
    pub(crate) onopen: Option<Rc<RefCell<dyn FnMut(Event)>>>,

    /// User supplied `error` handler.
    pub(crate) onerror: Option<Rc<RefCell<dyn FnMut(Event)>>>,

    /// User supplied `close` handler.
    pub(crate) onclose: Option<Rc<RefCell<dyn FnMut(Event)>>>,

    /// Reconnection config used for driving the exponential backoff reconnect system.
    pub(crate) reconnect: Option<Rc<RefCell<ReconnectConfig>>>,

    /// A storage location for EventHandlers.
    pub(crate) cb_store: Rc<RefCell<Vec<Option<Closure<dyn FnMut(Event) + 'static>>>>>,

    /// A switch to supppress reconnect behavior for when the WebSocket is closing.
    pub(crate) is_closing: Rc<RefCell<bool>>,
}

impl WebSocketBuilder {
    /// Create a new instance.
    pub(crate) fn new(url: Cow<'static, str>) -> Self {
        Self{
            url: Rc::new(url),
            protocols: None,
            onmessage: None,
            onopen: None,
            onerror: None,
            onclose: None,
            reconnect: Some(Rc::new(RefCell::new(ReconnectConfig::default()))),
            cb_store: Rc::new(RefCell::new(vec![])),
            is_closing: Rc::new(RefCell::new(false)),
        }
    }

    /// Finalize the build process and return the new WebSocket instance.
    ///
    /// ### Errors
    /// According to the [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/WebSocket#Exceptions_thrown)
    /// an error will only be returned if "The port to which the connection is being attempted is
    /// being blocked".
    pub fn build(self) -> Result<WebSocket, JsValue> {
        // Build the initial WebSocket instance.
        let ws = Rc::new(RefCell::new(
            WebSocketCore::build_new_websocket(&self.url, &self.protocols)?
        ));
        let core = WebSocketCore::new(self, ws);
        Ok(WebSocket::new(core))
    }

    /// Set a handler for the WebSocket's `message` event.
    ///
    /// The given closure will be called with the payload of the received WebSocket message frame.
    /// The contents of the frame will be placed in a `WsMessage` enum variant matching the
    /// `opcode` of the received frame. `WsMessage::Text(_)` for text frames and
    /// `WsMessage::Binary(_)` for binary frames.
    ///
    /// See [RFC 6455 1.2](https://tools.ietf.org/html/rfc6455#section-1.2) for more details on
    /// the WebSocket framing protocol.
    pub fn onmessage(mut self, f: impl FnMut(WsMessage) + 'static) -> Self {
        self.onmessage = Some(Rc::new(RefCell::new(f)));
        self
    }

    /// Set a handler for the WebSocket's `open` event.
    pub fn onopen(mut self, f: impl FnMut(Event) + 'static) -> Self {
        self.onopen = Some(Rc::new(RefCell::new(f)));
        self
    }

    /// Set a handler for the WebSocket's `error` event.
    pub fn onerror(mut self, f: impl FnMut(Event) + 'static) -> Self {
        self.onerror = Some(Rc::new(RefCell::new(f)));
        self
    }

    /// Set a handler for the WebSocket's `close` event.
    pub fn onclose(mut self, f: impl FnMut(Event) + 'static) -> Self {
        self.onclose = Some(Rc::new(RefCell::new(f)));
        self
    }

    /// The set of subprotocols to use for this connection.
    ///
    /// See [RFC 6455 1.9](https://tools.ietf.org/html/rfc6455#section-1.9) for more
    /// details on subprotocols.
    pub fn protocols<I>(mut self, protos: I) -> Self
        where
            I: Iterator,
            I::Item: Into<Cow<'static, str>>,
    {
        self.protocols = Some(Rc::new(protos.map(|s| s.into()).collect()));
        self
    }

    /// Overwrite the default reconnect config with some custom config.
    ///
    /// Gloo WebSockets are configured to reconnect by default, and will use the default value
    /// from a call to `ReconnectConfig::default()`. Overwrite the default with this method.
    ///
    /// If you want to supress reconnect behavior altogether, you should call this instance's
    /// `no_reconnect()` method.
    pub fn reconnect(mut self, cfg: ReconnectConfig) -> Self {
        self.reconnect = Some(Rc::new(RefCell::new(cfg)));
        self
    }

    /// Supress reconnect behavior on the created WebSocket.
    pub fn no_reconnect(mut self) -> Self {
        self.reconnect = None;
        self
    }
}
