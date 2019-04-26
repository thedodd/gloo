//! Common types used by the mid-level callbacks API & the high-level futures API.

use std::{
    cell::RefCell,
    rc::Rc,
    time::Duration,
};

use backoff::{
    backoff::Backoff,
    ExponentialBackoff,
};
use wasm_bindgen::closure::Closure;
use web_sys::Event;

/// Event variants which may come from an active WebSocket.
#[derive(Clone, Debug)]
pub enum WsEvent {
    Open(Event),
    Message(WsMessage),
    Error(Event),
    Close(Event),
}

/// Message variants which may be sent or received on a WebSocket.
#[derive(Clone, Debug)]
pub enum WsMessage {
    Text(String),
    Binary(Vec<u8>),
}

/// Variants of a WebSocket's connection state.
#[derive(Copy, Clone, Debug)]
pub enum ReadyState {
    Connecting,
    Open,
    Closing,
    Closed,
    /// Practically speaking, it is very unlikely that this variant will ever be encountered.
    Other(u16), // TODO: maybe we just use `unreachable!()` here.
}

impl From<u16> for ReadyState {
    /// Perform the conversion.
    ///
    /// [MDN Documentation](https://developer.mozilla.org/en-US/docs/Web/API/WebSocket/readyState)
    fn from(state: u16) -> Self {
        match state {
            0 => ReadyState::Connecting,
            1 => ReadyState::Open,
            2 => ReadyState::Closing,
            3 => ReadyState::Closed,
            _ => ReadyState::Other(state), // TODO: maybe we just use `unreachable!()` here.
        }
    }
}

/// Configuration for the exponential backoff reconnect system.
///
/// Consumers of this type will call its `next_backoff` method to being a reconnect process. The
/// internal state of the reconnect config will be updated as needed. Once a connection has been
/// successfully re-established, the `reset` method should be called, which will reset the internal
/// state of the instance.
#[derive(Debug)]
pub struct ReconnectConfig {
    is_reconnecting: bool,
    backoff: ExponentialBackoff,
    retry_closure: Rc<RefCell<Option<Closure<dyn FnMut() + 'static>>>>,
}

impl ReconnectConfig {
    /// Create a new instance with the default configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new instance with full control over the backoff algorithm's primary parameters.
    ///
    /// #### multiplier
    /// The value to multiply the current interval with for each successive retry attempt.
    ///
    /// #### randomization factor
    /// The randomization factor to use for creating a range around the retry interval.
    /// A value of `0.5` results in a random period ranging between 50% below and
    /// 50% above the retry interval.
    ///
    /// #### max interval
    /// The maximum value of the backoff period. Once the retry interval reaches this value
    /// it stops increasing until `reset` is called.
    pub fn new_custom(randomization_factor: f64, multiplier: f64, max_interval: Duration) -> Self {
        let mut val = Self::new();
        val.backoff.randomization_factor = randomization_factor;
        val.backoff.multiplier = multiplier;
        val.backoff.max_interval = max_interval;
        val
    }

    /// Check if this config is currently in a reconnecting state.
    ///
    /// This is used to to indicate if that backoff configuration is being used via the
    /// `self.next_backoff()` method.
    pub fn is_reconnecting(&self) -> bool {
        self.is_reconnecting
    }

    /// Get the duration to be awaited until another reconnect attempt should be made.
    ///
    /// Calling this method will transition the state of this instance to a `reconnecting` state,
    /// and will internally update its configuration such that the next call to `next_backoff`
    /// will return a duration compliant with the instance's exponential backoff algorithm.
    pub fn next_backoff(&mut self) -> Duration {
        // NOTE: a `None` value would only every be returned if `backoff.max_elapsed_time` was
        // exceeded; however, we restrict the configuration of this value, so it will never be hit.
        self.backoff.next_backoff().unwrap_or_default()
    }

    /// Reset this instances internal state.
    ///
    /// It will no longer be reckoned as being in a reconnecting state (until the next time
    /// `next_backoff` is called), and the backoff algorithm's settings will be reset.
    pub fn reset(&mut self) {
        self.is_reconnecting = false;
        self.backoff.reset();
    }

    /// Update the retry cb tracked by this instance.
    pub(crate) fn set_retry_cb(&self, cb: Closure<dyn FnMut() + 'static>) {
        self.retry_closure.borrow_mut().replace(cb);
    }
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        let mut backoff = ExponentialBackoff::default();
        backoff.max_interval = Duration::from_secs(60); // One minute max interval.
        backoff.multiplier = 1.5; // Increase by 50% each interval.
        backoff.max_elapsed_time = None; // Never allow max_elapsed_time.
        let retry_closure = Rc::new(RefCell::new(None));
        ReconnectConfig{is_reconnecting: false, backoff, retry_closure}
    }
}
