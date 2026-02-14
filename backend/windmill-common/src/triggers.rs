use lazy_static::lazy_static;
use quick_cache::sync::Cache;

pub use windmill_types::triggers::*;

lazy_static! {
    pub static ref RUNNABLE_FORMAT_VERSION_CACHE: Cache<RunnableFormatCacheKey, RunnableFormat> =
        Cache::new(1000);
}
