use derive_more::Debug;
use eframe::egui;
use std::{
    borrow::Borrow,
    future::Future,
    sync::{Arc, Mutex},
    time::Duration,
};
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

pub trait Tickable {
    fn tick(&mut self, ctx: &egui::Context) -> Duration;
}

/// A legendarily cringe hack to have background updates until next major
/// version of eframe finally has those
#[derive(Debug)]
pub struct UpdatableApp<T>(Arc<Mutex<T>>);

impl<T> UpdatableApp<T>
where
    T: eframe::App + Tickable + Send + 'static,
{
    pub fn new(app: T, ctx: &egui::Context) -> Self {
        let data = Arc::new(Mutex::new(app));
        let ret = data.clone();

        let ctx = ctx.clone();
        tokio::spawn(async move {
            loop {
                let sleep = data.lock().unwrap().tick(&ctx);
                tokio::time::sleep(sleep).await;
            }
        });

        Self(ret)
    }
}

impl<T: eframe::App> eframe::App for UpdatableApp<T> {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.0.lock().unwrap().update(ctx, frame)
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        self.0.lock().unwrap().save(storage)
    }
}

// lol
// does the * 2 thing mean those strings use 2x the space
// or is it smart enough to figure out we're only using the slice
// idk, and it's negligible anyway
#[allow(unused_macros)] // same as above
macro_rules! to_title_case {
    ($s:expr) => {{
        const B: &[u8] = $s.as_bytes();
        const BUF: ([u8; B.len() * 2], usize) = {
            let mut buf = [0; B.len() * 2];
            let mut buf_pos = 0;
            let mut i = 0;
            while i < B.len() {
                let next = B[i];
                if i != 0 && next.is_ascii_uppercase() {
                    buf[buf_pos] = b' ';
                    buf_pos += 1;
                }
                buf[buf_pos] = next;
                buf_pos += 1;
                i += 1;
            }
            (buf, buf_pos)
        };
        // We only insert ascii spaces before an ascii uppercase,
        // which means we insert a valid utf8 codepoint in between utf8
        // codepoints - meaning the result is always valid utf8
        // Could use from_utf8_unchecked but this is run at compile time anyway
        match std::str::from_utf8(&BUF.0.split_at(BUF.1).0) {
            ::std::result::Result::Ok(s) => s,
            ::std::result::Result::Err(_) => panic!("to_title_case! failed somehow"),
        }
    }};
}
#[allow(unused_imports)] // same as above
pub(crate) use to_title_case;

#[cfg(test)]
#[test]
fn test_const_title_case() {
    // happy path
    assert_eq!(to_title_case!("SomeFunkyTool"), "Some Funky Tool");

    // see that this compiles (meaning the * 2 trick worked lol)
    const SCHLONG: &str = to_title_case!(
        "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA"
    );
    println!("{SCHLONG}");

    // unicode gibberish compiles too
    to_title_case!("𐍈𓂀ܰᚦΞ𝔄꧁৹ဨ");

    // unicode gibberish but with an ascii uppercase in it
    const P: &str = to_title_case!("𐍈𓂀ܰᚦΞB𝔄꧁৹ဨ");
    println!("{P}")
}
