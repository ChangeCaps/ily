use std::{
    any::Any,
    fmt::Debug,
    sync::{Arc, Mutex},
};

use crate::Event;

/// An event sender, that can send events to the application.
///
/// This is usually implemented by the application shell and
/// should not be implemented by the user.
pub trait EventEmitter: Send + 'static {
    fn send_event(&mut self, event: Event);
}

impl EventEmitter for () {
    fn send_event(&mut self, _: Event) {}
}

/// An event sink, that can send events to the application.
#[derive(Clone)]
pub struct EventSink {
    emitter: Arc<Mutex<dyn EventEmitter>>,
}

impl EventSink {
    /// Creates a dummy event sink, that does nothing.
    pub fn dummy() -> Self {
        Self::new(())
    }

    /// Creates a new event sink from an [`EventSender`].
    pub fn new(sender: impl EventEmitter) -> Self {
        Self {
            emitter: Arc::new(Mutex::new(sender)),
        }
    }

    /// Sends an event to the application.
    pub fn emit(&self, event: impl Any + Send + Sync) {
        self.emitter.lock().unwrap().send_event(Event::new(event));
    }
}

impl Debug for EventSink {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EventSink").finish()
    }
}
