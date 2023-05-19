use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    panic::Location,
};

use crate::{Callback, CallbackEmitter, Resource, Sendable};

use super::effect;

pub struct ReadSignal<T: Sendable + 'static> {
    pub(crate) resource: Resource<T>,
    pub(crate) emitter: Resource<CallbackEmitter>,
}

impl<T: Sendable + 'static> ReadSignal<T> {
    pub fn new_leaking(value: T) -> Self {
        Self {
            resource: Resource::new_leaking(value),
            emitter: Resource::new_leaking(CallbackEmitter::new()),
        }
    }

    pub fn reference(self) {
        self.resource.reference();
        self.emitter.reference();
    }

    pub fn try_get(self) -> Option<T>
    where
        T: Clone,
    {
        self.resource.get()
    }

    #[track_caller]
    pub fn get_untracked(self) -> T
    where
        T: Clone,
    {
        match self.try_get() {
            Some(value) => value,
            None => panic!(
                "Signal::get() called on a dropped signal {:?}",
                self.resource.id()
            ),
        }
    }

    #[track_caller]
    pub fn get(self) -> T
    where
        T: Clone,
    {
        self.track();
        self.get_untracked()
    }

    pub fn track(self) {
        if let Some(emitter) = self.emitter.get() {
            effect::track_callback(emitter.downgrade());
        }
    }

    pub fn emitter(self) -> Option<CallbackEmitter> {
        self.emitter.get()
    }

    pub fn subscribe(self, callback: &Callback) {
        if let Some(emitter) = self.emitter.get() {
            emitter.subscribe(callback);
        }
    }
}

impl<T: Sendable> Clone for ReadSignal<T> {
    fn clone(&self) -> Self {
        Self {
            resource: self.resource.clone(),
            emitter: self.emitter.clone(),
        }
    }
}

impl<T: Sendable> Copy for ReadSignal<T> {}

pub struct Signal<T: Sendable + 'static> {
    signal: ReadSignal<T>,
}

impl<T: Sendable> Deref for Signal<T> {
    type Target = ReadSignal<T>;

    fn deref(&self) -> &Self::Target {
        &self.signal
    }
}

impl<T: Sendable + 'static> Signal<T> {
    pub fn new_leaking(value: T) -> Self {
        Self {
            signal: ReadSignal::new_leaking(value),
        }
    }

    #[track_caller]
    pub fn set(self, value: T) {
        if self.try_set(value).is_err() {
            panic!("Signal::set() called on a dropped signal");
        }
    }

    #[track_caller]
    pub fn try_set(self, value: T) -> Result<(), T> {
        self.try_set_untracked(value)?;
        self.emit();
        Ok(())
    }

    #[track_caller]
    pub fn set_untracked(self, value: T) {
        if self.try_set_untracked(value).is_err() {
            panic!("Signal::set_untracked() called on a dropped signal");
        }
    }

    #[track_caller]
    pub fn try_set_untracked(self, value: T) -> Result<(), T> {
        match self.signal.resource.set(value) {
            Ok(_) => Ok(()),
            Err(value) => Err(value),
        }
    }

    pub fn modify(self) -> Modify<T>
    where
        T: Clone,
    {
        Modify::new(self)
    }

    #[track_caller]
    pub fn emit(self) {
        if let Some(emitter) = self.signal.emitter.get() {
            tracing::trace!("emitting signal at {}", Location::caller());
            emitter.clear_and_emit(&());
        }
    }
}

impl<T: Sendable> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            signal: self.signal,
        }
    }
}

impl<T: Sendable> Copy for Signal<T> {}

pub struct Modify<T: Sendable + Clone + 'static> {
    signal: Signal<T>,
    value: Option<T>,
}

impl<T: Sendable + Clone> Modify<T> {
    pub fn new(signal: Signal<T>) -> Self {
        Self {
            signal,
            value: Some(signal.get()),
        }
    }
}

impl<T: Sendable + Clone> Deref for Modify<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.value.as_ref().unwrap()
    }
}

impl<T: Sendable + Clone> DerefMut for Modify<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.value.as_mut().unwrap()
    }
}

impl<T: Sendable + Clone> Drop for Modify<T> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.signal.set(value);
        }
    }
}

/// A signal that owns its resources.
///
/// This is useful for signals that aren't bound to a [`Scope`].
pub struct OwnedSignal<T: Sendable + 'static> {
    signal: Signal<T>,
}

impl<T: Sendable> Deref for OwnedSignal<T> {
    type Target = Signal<T>;

    fn deref(&self) -> &Self::Target {
        &self.signal
    }
}

impl<T: Sendable> Clone for OwnedSignal<T> {
    fn clone(&self) -> Self {
        self.reference();

        Self {
            signal: self.signal,
        }
    }
}

impl<T: Sendable + Default> Default for OwnedSignal<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Sendable> OwnedSignal<T> {
    pub fn new(value: T) -> Self {
        Self {
            signal: Signal::new_leaking(value),
        }
    }

    pub fn bind(&mut self, signal: Signal<T>) {
        self.signal = signal;
        self.reference();
    }
}

impl<T: Sendable> From<T> for OwnedSignal<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T: Sendable> Drop for OwnedSignal<T> {
    #[track_caller]
    fn drop(&mut self) {
        self.resource.dispose();
        self.emitter.dispose();
    }
}

macro_rules! impl_signal {
    ($($type:ty),*) => {
        $(
            impl<T: Sendable + Clone + Debug> Debug for $type {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    f.debug_struct(stringify!($type))
                        .field("resource", &self.resource)
                        .field("emitter", &self.emitter)
                        .finish()
                }
            }

            impl<T: Sendable + Clone + PartialEq> PartialEq for $type {
                fn eq(&self, other: &Self) -> bool {
                    self.resource == other.resource
                }
            }

            impl<T: Sendable + Clone + Eq> Eq for $type {}
        )*
    };
}

impl_signal!(ReadSignal<T>, Signal<T>, OwnedSignal<T>);
