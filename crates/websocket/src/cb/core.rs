use std::{
    borrow::Cow,
    cell::{RefCell},
    rc::Rc,
};

use js_sys::{
    Array,
    ArrayBuffer,
    Function,
    JsString,
    Uint8Array,
};
use wasm_bindgen::{
    JsCast, JsValue,
    closure::Closure,
};
use web_sys::{self, BinaryType, Event, MessageEvent};

use crate::{
    cb::builder::WebSocketBuilder,
    common::WsMessage,
};

/// The core of the Gloo WebSocket abstraction.
///
/// This type encapsulates all of the low-level logic related to interfacing with the wasm-bindgen
/// and web-sys components needed for building new WebSocket instances, handling reconnect logic
/// and the like.
pub(crate) struct WebSocketCore {
    pub builder: Rc<WebSocketBuilder>,
    pub ws: Rc<RefCell<web_sys::WebSocket>>,
}

impl WebSocketCore {
    /// Create a new `web_sys::WebSocket` instance.
    pub(crate) fn build_new_websocket(url: &Cow<'static, str>, protocols: &Option<Rc<Vec<Cow<'static, str>>>>) -> Result<web_sys::WebSocket, JsValue> {
        let ws = match protocols {
            Some(protos) => {
                let js_protos = protos.iter().fold(Array::new(), |acc, proto| {
                    acc.push(&proto.as_ref().into());
                    acc
                });
                web_sys::WebSocket::new_with_str_sequence(url.as_ref(), &js_protos.into())?
            }
            None => {
                web_sys::WebSocket::new(url.as_ref())?
            }
        };
        ws.set_binary_type(BinaryType::Arraybuffer);
        Ok(ws)
    }

    /// Create a new instance.
    ///
    /// This routine will initialize all handlers needed on the given WebSocket instance.
    pub(crate) fn new(builder: WebSocketBuilder, ws: Rc<RefCell<web_sys::WebSocket>>) -> Self {
        let builder = Rc::new(builder);
        Self::init_new_websocket(builder.clone(), ws.clone());
        Self{builder, ws}
    }

    /// Close the WebSocket connection and suppress reconnect behavior as needed.
    pub(crate) fn close(&self, code: u16, reason: Option<String>) -> Result<(), JsValue> {
        *self.builder.is_closing.borrow_mut() = true;
        let ws = self.ws.borrow();
        match reason {
            None => ws.close_with_code(code),
            Some(reason) => ws.close_with_code_and_reason(code, reason.as_str()),
        }
    }

    //////////////////////////////////////////////////////////////////////////
    // Private Interface /////////////////////////////////////////////////////

    /// Build the callback which is used to handle `message` events.
    fn build_onmessage(builder: Rc<WebSocketBuilder>) -> Option<Closure<dyn FnMut(Event) + 'static>> {
        // Unpack the user supplied value. If none, we have nothing to do.
        let cb = match builder.onmessage.clone() {
            None => return None,
            Some(cb) => cb.clone(),
        };

        Some(Closure::wrap(Box::new(move |event: Event| {
            // This will always be a `MessageEvent` instance, so extract its data.
            let event: MessageEvent = event.unchecked_into();
            let data = event.data();

            // Extract text data if this is a text payload.
            if let Some(val) = JsString::try_from(&data) {
                let inner_cb = &mut *cb.borrow_mut();
                return inner_cb(WsMessage::Text(String::from(val)));
            }

            // The given payload was not a text payload, so it will be an `ArrayBuffer` due to
            // setting the WebSocket's binary type to `ArrayBuffer`.
            let arr_buf: ArrayBuffer = data.unchecked_into();
            let u8buf = Uint8Array::new(&arr_buf); // This is binary data, so treat it as `Uint8Array`.
            let mut decode_buf = vec![0; u8buf.byte_length() as usize]; // We need to use exact length to avoid panic (as of 2019.04.05).
            u8buf.copy_to(&mut decode_buf);
            let inner_cb = &mut *cb.borrow_mut();
            inner_cb(WsMessage::Binary(decode_buf));
        })))
    }

    /// Build the callback which is used to handle `open` events.
    fn build_onopen(builder: Rc<WebSocketBuilder>) -> Option<Closure<dyn FnMut(Event) + 'static>> {

        // If the builder is not configured to reconnect, and no `open` handler has been provided
        // then there is nothing to do here.
        if builder.onopen.is_none() && builder.reconnect.is_none() {
            return None;
        }

        Some(Closure::wrap(Box::new(move |event: Event| {
            // Update our reconnect config if needed.
            if let Some(cfg) = builder.reconnect.clone() {
                cfg.borrow_mut().reset();
            }

            // Pass event to user's callback if needed.
            if let Some(cb) = builder.onopen.clone() {
                let inner_cb = &mut *cb.borrow_mut();
                inner_cb(event);
            }
        })))
    }

    /// Build the callback which is used to handle `error` events.
    fn build_onerror(builder: Rc<WebSocketBuilder>) -> Option<Closure<dyn FnMut(Event) + 'static>> {
        // Unpack the user supplied value. If none, we have nothing to do.
        let cb = match builder.onerror.clone() {
            None => return None,
            Some(cb) => cb,
        };

        Some(Closure::wrap(Box::new(move |event: Event| {
            let inner_cb = &mut *cb.borrow_mut();
            inner_cb(event);
        })))
    }

    /// Build the callback which is used to handle `close` events.
    ///
    /// For reconnecting instances, the returned closure will retain an Rc reference to various
    /// callbacks supplied by the user. When a new WebSocket instance is build, the closures will
    /// be bound as event handlers on the new instance.
    fn build_onclose(builder: Rc<WebSocketBuilder>, ws: Rc<RefCell<web_sys::WebSocket>>) -> Option<Closure<FnMut(Event) + 'static>> {
        // If the builder is not configured to reconnect, and no `close` handler has been provided
        // then there is nothing to do here.
        if builder.onclose.is_none() && builder.reconnect.is_none() {
            return None;
        }

        Some(Closure::wrap(Box::new(move |event: Event| {
            // If this instance is configured to reconnect, and a reconnect is now needed, setup
            // retry callbacks via backoff config and timeout callbacks.
            if !*builder.is_closing.borrow() {
                if let Some(cfg) = builder.reconnect.clone() {
                    // Gather items needed for scheduling a retry callback.
                    let next_backoff = cfg.borrow_mut().next_backoff();
                    let retry_cb = Self::build_retry_closure(builder.clone(), ws.clone());

                    // Schedule the retry callback.
                    Self::schedule_reconnect(
                        retry_cb.as_ref().unchecked_ref(),
                        next_backoff.as_secs() as i32, // u64 -> i32 will be truncated if larger than i32.
                    );

                    // Store our retry callback to ensure it is not prematurely dropped.
                    cfg.borrow_mut().set_retry_cb(retry_cb);
                }
            }

            // Pass event to user's callback if needed.
            if let Some(cb) = builder.onclose.clone() {
                let inner_cb = &mut *cb.borrow_mut();
                inner_cb(event);
            }
        })))
    }

    /// Build a retry closure to be executed at some point in the future.
    fn build_retry_closure(builder: Rc<WebSocketBuilder>, ws: Rc<RefCell<web_sys::WebSocket>>) -> Closure<dyn FnMut() + 'static> {
        Closure::wrap(Box::new(move || {
            // If the WebSocket was manually closed by a user before this callback was invoked,
            // the suppress the reconnect behavior.
            if *builder.is_closing.borrow() {
                return;
            }

            // When this retry closure is invoked, build a new WebSocket instance.
            let new_ws = match Self::build_new_websocket(&builder.url, &builder.protocols) {
                Ok(new_ws) => new_ws,
                // If this branch is hit, then a new retry callback must be scheduled.
                Err(_) => {
                    // Gather items needed for scheduling a retry callback.
                    let cfg = builder.reconnect.clone().unwrap(); // Safe. Only here if already in reconnect flow.
                    let next_backoff = cfg.borrow_mut().next_backoff();
                    let retry_cb = Self::build_retry_closure(builder.clone(), ws.clone());

                    // Schedule the retry callback.
                    Self::schedule_reconnect(
                        retry_cb.as_ref().unchecked_ref(),
                        next_backoff.as_secs() as i32, // u64 -> i32 will be truncated if larger than i32.
                    );

                    // Store our retry callback to ensure it is not prematurely dropped.
                    cfg.borrow_mut().set_retry_cb(retry_cb);
                    return;
                },
            };

            // Replace old WebSocket instance with the new instance.
            {
                *ws.borrow_mut() = new_ws;
            }

            // Initialize new instance.
            Self::init_new_websocket(builder.clone(), ws.clone());
        }))
    }

    /// Initialize a new web_sys::WebSocket with needed event handlers.
    fn init_new_websocket(builder: Rc<WebSocketBuilder>, ws: Rc<RefCell<web_sys::WebSocket>>) {
        // Build the mid-level API callbacks which wrap the user given callbacks.
        // TODO: wrap these closures in gloo event handlers once rustwasm/gloo#42 lands.
        let onmessage = Self::build_onmessage(builder.clone());
        let onopen = Self::build_onopen(builder.clone());
        let onerror = Self::build_onerror(builder.clone());
        let onclose = Self::build_onclose(builder.clone(), ws.clone());

        // Register the generated event handlers. **NOTE WELL:** this is done once here in the
        // constructor; however, for reconnecting instances, this will also be done for new
        // WebSocket connections generated by way of the retry closures from `build_retry_closure`.
        {
            let inner_ws = ws.as_ref().borrow();
            inner_ws.set_onmessage(onmessage.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
            inner_ws.set_onopen(onopen.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
            inner_ws.set_onerror(onerror.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
            inner_ws.set_onclose(onclose.as_ref().map(|cb| cb.as_ref().unchecked_ref()));
        }

        // Clear out old event handlers, and store the new event handlers.
        {
            let mut store = builder.cb_store.borrow_mut();
            store.clear();
            store.push(onmessage);
            store.push(onopen);
            store.push(onerror);
            store.push(onclose);
        }
    }

    /// Schedule the given function to be executed at the given timeout.
    fn schedule_reconnect(func: &Function, timeout: i32) {
        web_sys::window()
            .unwrap() // Access to the window should normally not be problematic.
            .set_timeout_with_callback_and_timeout_and_arguments_0(func, timeout)
            .unwrap(); // This should be safe under normal circumstances.
    }
}
