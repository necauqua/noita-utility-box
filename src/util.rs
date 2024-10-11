use derive_more::Debug;
use std::{borrow::Borrow, future::Future};
use tokio::sync::oneshot::{self, error::TryRecvError, Receiver};

/// A variant of poll-promise that can be used as storage. Uses tokio.
#[derive(Debug)]
pub enum Promise<T> {
    Pending(Receiver<T>),
    Done(T),
    Taken,
}

// this could happen when the tokio runtime shuts down ig
fn no_sender() -> ! {
    panic!("Promise sender dropped");
}

impl<T> Promise<T> {
    pub fn spawn<F>(future: F) -> Self
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        // we use tokio and not pollster or something because
        // obws brings (and depends on) tokio anyway
        let (tx, rx) = oneshot::channel();
        tokio::spawn(async { tx.send(future.await) });
        Self::Pending(rx)
    }

    /// Borrow the value if the promise is complete, otherwise return None.
    /// Panics if the promise value was taken.
    pub fn poll<Q>(&mut self) -> Option<&Q>
    where
        Q: ?Sized,
        T: Borrow<Q>,
    {
        match self {
            Promise::Pending(rx) => match rx.try_recv() {
                Ok(t) => {
                    *self = Promise::Done(t);
                    // recurse into the outer match lol
                    self.poll()
                }
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Closed) => no_sender(),
            },
            Promise::Done(ref t) => Some(t.borrow()),
            Promise::Taken => panic!("Promise was taken"),
        }
    }

    /// A shorthand for `self.poll().unwrap_or_default()`.
    pub fn poll_or_default<Q>(&mut self) -> &Q
    where
        Q: ?Sized,
        T: Borrow<Q> + Default,
        for<'a> &'a Q: Default,
    {
        self.poll().unwrap_or_default()
    }

    /// Take the value if the promise is complete, otherwise return None.
    /// Subsequent calls to `poll_take` or `poll` will panic.
    pub fn poll_take(&mut self) -> Option<T> {
        match std::mem::replace(self, Promise::Taken) {
            Promise::Pending(mut rx) => match rx.try_recv() {
                Ok(t) => Some(t),
                Err(TryRecvError::Empty) => {
                    *self = Promise::Pending(rx);
                    None
                }
                Err(TryRecvError::Closed) => no_sender(),
            },
            Promise::Done(t) => Some(t),
            Promise::Taken => panic!("Promise was already taken"),
        }
    }

    pub fn is_taken(&self) -> bool {
        matches!(self, Promise::Taken)
    }
}

impl<T: Default> Default for Promise<T> {
    fn default() -> Self {
        Self::Done(Default::default())
    }
}

/// Implement [serde::Serialize] and [serde::Deserialize] for a struct, only
/// writing/reading the specified fields and using Default when reading.
#[allow(unused_macros)] // false positive?. it's definitely used
macro_rules! persist {
    (__ref_of $lt:lifetime, String) => {
        &$lt str
    };
    (__ref_of $lt:lifetime,$t:ty) => {
        &$lt $t
    };
    ($t:ident { $($field:ident: $field_t:ty),* $(,)? }) => {
        impl ::serde::Serialize for $t {
            fn serialize<S: ::serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {

                #[derive(::serde::Serialize)]
                struct Persisted<'a> {
                    $($field: persist!(__ref_of 'a, $field_t),)*
                    #[serde(skip)]
                    _phantom: ::std::marker::PhantomData<&'a ()>,
                }

                Persisted { $($field: &self.$field,)* _phantom: ::std::marker::PhantomData }.serialize(serializer)
            }
        }
        impl<'de> ::serde::Deserialize<'de> for $t {
            fn deserialize<D: ::serde::Deserializer<'de>>(
                deserializer: D,
            ) -> Result<Self, D::Error> {
                #[derive(::serde::Deserialize)]
                struct Persisted {
                    $($field: $field_t,)*
                }
                let _persisted = Persisted::deserialize(deserializer)?;
                #[allow(clippy::needless_update)]
                ::std::result::Result::Ok($t {
                    $($field: _persisted.$field,)*
                    ..Default::default()
                })
            }
        }
    };
}

#[allow(unused_imports)] // same as above
pub(crate) use persist;
