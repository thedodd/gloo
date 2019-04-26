// //! A high-level futures-based API for Websockets built on top of the wasm-bindgen ecosystem.
// //!
// //! ### TODO
// //! - [ ] need to experiment with error conditions on initial connections and reconnects.
// //!   Hopefully browsers will exhibit the same behavior in terms of the order of events emitted
// //!   and the like, not sure at this point.
// //! - [ ] need to ensure that reconnect backoff logic works as expected.

// use std::rc::Rc;

// use futures::{
//     prelude::*,
//     Async, AsyncSink,
//     sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender},
// };
// use js_sys::{ArrayBuffer, Uint8Array};
// use wasm_bindgen::{JsValue, JsCast};
// use web_sys::{self, BinaryType, Event, MessageEvent};

// use crate::{
//     common::{
//         WsEvent,
//         WsMessage,
//         ReadyState,
//         ReconnectState,
//     },
// };

// /// A WebSocket type offering a reconnect functionality and a `Stream + Sink` interface.
// #[derive(Debug)]
// pub struct WebSocket {
//     /// The underlying WebSocket instance.
//     ///
//     /// If this instance is configured to reconnect, this web-sys::WebSocket will be swapped out
//     /// on reconnects.
//     ws: web_sys::WebSocket,

//     /// The optional configuration for handling reconnects.
//     reconnect: Option<ReconnectState>,

//     /// The channel receiver used for streaming in the events from the underlying WebSocket.
//     receiver: UnboundedReceiver<WsEvent>,

//     /// The channel sender used for handling events related to this WebSocket.
//     ///
//     /// Used by the closures sent over to JS land for the web-sys WebSocket callbacks.
//     sender: UnboundedSender<WsEvent>,

//     /// An array of the already cast wasm-bindgen closures used internally by this type.
//     ///
//     /// Their ordering is as follows:
//     ///
//     /// 1. on_message
//     /// 2. on_open
//     /// 3. on_error
//     /// 4. on_close
//     ///
//     /// **NB:** The ordering here is very important. In order to avoid having to recast the
//     /// various closures when we need to reconnect, we store the 4 different closures as
//     /// `Rc<js_sys::Function>`s and then we ensure that we pass them to the appropriate handlers
//     /// during reconnect.
//     callbacks_internal: [Rc<js_sys::Function>; 4],
// }

// impl WebSocket {
//     /// Create a WebSocket connection to the taget URL. No reconnects will be performed.
//     pub fn connect<U: AsRef<str>>(url: U) -> Result<Self, JsValue> {
//         let mut ws = web_sys::WebSocket::new(url.as_ref())?;
//         let (sender, receiver) = unbounded::<WsEvent>();
//         ws.set_binary_type(BinaryType::Arraybuffer);
//         let callbacks = Self::build_callbacks(sender.clone());
//         // ws.set_onmessage()
//         // ws.set_onopen()
//         // ws.set_onerror()
//         // ws.set_onclose()
//     }

//     // TODO: build this out.
//     // pub fn with_reconnect()

//     /// Build the callbacks needed for handling web_sys WebSocket events.
//     fn build_callbacks(sender: UnboundedSender<WsEvent>) -> [Rc<js_sys::Function>; 4] {

//     }

//     /// The current state of this WebSocket's connection.
//     pub fn ready_state(&self) -> ReadyState {
//         self.ws.ready_state().into()
//     }

//     /// Attempt to reconnect this websocket.
//     ///
//     /// NB: this method is invoked by way of receiving a WsEvent::Reconnect on this instances
//     /// receiver. This is the only mechanism which should trigger this method being called.
//     /// Scheduling of reconnects is encapsulated by this type in such a way that it honors the
//     /// configured exponential backoff settings. This should be upheld throughout.
//     fn reconnect(&mut self) {
//         // TODO: finish this up.
//     }
// }

// impl Sink for WebSocket {
//     type SinkItem = WsMessage;
//     type SinkError = ();

//     fn start_send(&mut self, mut item: Self::SinkItem) -> StartSend<Self::SinkItem, Self::SinkError> {
//         // Handle connecting or disconnected states. If the instance is not configured to
//         // reconnect, then we must return an error on disconnected state.
//         match self.ready_state() {
//             ReadyState::Open => (),
//             // We're not ready to write yet, have caller try again later.
//             ReadyState::Connecting => return Ok(AsyncSink::NotReady(item)),
//             ReadyState::Closing | ReadyState::Closed | ReadyState::Other(_) => match &mut self.reconnect {
//                 // We're disconnected. If this instance is not configured to reconnect,
//                 // then this is an error and the Sink should be reckoned as closed.
//                 None => return Err(()),
//                 Some(reconnect) => match reconnect.is_in_progress {
//                     // A reconnect is already in progress, so return not ready.
//                     true => return Ok(AsyncSink::NotReady(item)),
//                     // No reconnect has been scheduled yet, so schedule one.
//                     false => {
//                         reconnect.is_in_progress = true;
//                         // This unwrap will never fail as we are holding open both ends of the channel.
//                         self.sender.unbounded_send(WsEvent::Reconnect).unwrap();
//                         return Ok(AsyncSink::NotReady(item));
//                     }
//                 }
//             }
//         };

//         // We are ready to send the given message.
//         let res = match &mut item {
//             WsMessage::Text(msg) => self.ws.send_with_str(msg.as_str()),
//             WsMessage::Binary(msg) => self.ws.send_with_u8_array(msg.as_mut_slice()),
//         };

//         // Handle errors coming from JS land.
//         match res {
//             Err(_) => match &self.reconnect {
//                 Some(_) => return Ok(AsyncSink::NotReady(item)),
//                 None => {
//                     // The error condition is pretty simple here, the socket is down.
//                     // This sink should now be reckoned as closed, per the Sink documentation:
//                     // https://docs.rs/futures/0.1.25/futures/sink/trait.Sink.html#errors
//                     return Err(());
//                 }
//             }
//             Ok(_) => Ok(AsyncSink::Ready),
//         }
//     }

//     fn poll_complete(&mut self) -> Poll<(), Self::SinkError> {
//         // Currently, there is no buffering, so we just return ready.
//         Ok(Async::Ready(()))
//     }
// }

// impl Stream for WebSocket {
//     type Item = WsEvent;
//     type Error = ();

//     fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
//         // Poll for the next item from the stream.
//         let event = match self.receiver.poll() {
//             Ok(Async::Ready(Some(event))) => event,
//             Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
//             Ok(Async::NotReady) => return Ok(Async::NotReady),
//             Err(err) => return Err(err),
//         };

//         // Evaluate the received event for driving reconnect logic before forwarding.
//         match &event {
//             // The initial or retried connection has been established. Updated our reconnect
//             // config if we have any.
//             WsEvent::Open(_) => match &mut self.reconnect {
//                 Some(reconnect) => {
//                     reconnect.is_in_progress = false;
//                     // TODO: reset backoff state.
//                 }
//                 None => (),
//             }
//             WsEvent::Error(_) => (), // Errors should be followed by a closed event in browsers.
//             WsEvent::Close(_) => match &mut self.reconnect {
//                 Some(reconnect) => match reconnect.is_in_progress {
//                     // Received a WebSocket close event before it ever went into an open state,
//                     // so we need to schedule a new reconnect event at an appropriate backoff.
//                     true => self.reconnect(),
//                     // Close event was received outside of a retry context.
//                     // We need to immediately schedule a new reconnect event.
//                     false => {
//                         reconnect.is_in_progress = true;
//                         // This unwrap will never fail as we are holding open both ends of the channel.
//                         self.sender.unbounded_send(WsEvent::Reconnect).unwrap();
//                     }
//                 }
//                 // This instance is not configured to reconnect, so we need to to close our channel.
//                 None => self.receiver.close(),
//             }
//             // We've received a reconnect event published from this instance.
//             WsEvent::Reconnect => self.reconnect(),
//             _ => (),
//         };

//         // Forward the polled message.
//         Ok(Async::Ready(Some(event)))
//     }
// }
