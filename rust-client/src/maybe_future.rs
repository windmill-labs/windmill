#[cfg(feature = "async")]
use futures::future::BoxFuture;

/// A conditional type that represents either:
/// - A boxed future (async mode)
/// - A direct value (sync mode)
pub mod maybe_future {
    #[cfg(feature = "async")]
    pub type MaybeFuture<'a, T> = futures::future::BoxFuture<'a, T>;
    #[cfg(not(feature = "async"))]
    pub type MaybeFuture<'a, T> = T;
}

/// Bridges between async and sync execution contexts
///
/// Behavior depends on compilation:
/// - With `async` feature: Returns a boxed future
/// - Without `async` feature: Blocks on the global runtime
#[macro_export]
macro_rules! ret {
    ($ex:expr) => {
        #[cfg(feature = "async")]
        return async move { $ex.await }.boxed();

        #[cfg(not(feature = "async"))]
        return crate::maybe_future::RUNTIME.block_on($ex);
    };
}

/// The global Tokio runtime instance used in sync mode
///
/// Initialized lazily with:
/// - I/O capabilities
/// - Time utilities
/// - Current-thread executor
///
/// # Panics
///
/// If Tokio fails to initialize the runtime
#[cfg(not(feature = "async"))]
pub(crate) static RUNTIME: once_cell::sync::Lazy<tokio::runtime::Runtime> =
    once_cell::sync::Lazy::new(|| {
        tokio::runtime::Builder::new_current_thread()
            .enable_io()
            .enable_time()
            .build()
            .unwrap()
    });
