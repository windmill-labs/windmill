// Licensed to the Apache Software Foundation (ASF) under one
// or more contributor license agreements.  See the NOTICE file
// distributed with this work for additional information
// regarding copyright ownership.  The ASF licenses this file
// to you under the Apache License, Version 2.0 (the
// "License"); you may not use this file except in compliance
// with the License.  You may obtain a copy of the License at
//
//   http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing,
// software distributed under the License is distributed on an
// "AS IS" BASIS, WITHOUT WARRANTIES OR CONDITIONS OF ANY
// KIND, either express or implied.  See the License for the
// specific language governing permissions and limitations
// under the License.

use rand::{prelude::*, rng};
use std::time::Duration;

/// Exponential backoff with decorrelated jitter algorithm
///
/// The first backoff will always be `init_backoff`.
///
/// Subsequent backoffs will pick a random value between `init_backoff` and
/// `base * previous` where `previous` is the duration of the previous backoff
///
/// See <https://aws.amazon.com/blogs/architecture/exponential-backoff-and-jitter/>
#[allow(missing_copy_implementations)]
#[derive(Debug, Clone)]
pub struct BackoffConfig {
    /// The initial backoff duration
    pub init_backoff: Duration,
    /// The maximum backoff duration
    pub max_backoff: Duration,
    /// The multiplier to use for the next backoff duration
    pub base: f64,
}

impl Default for BackoffConfig {
    fn default() -> Self {
        Self {
            init_backoff: Duration::from_millis(100),
            max_backoff: Duration::from_secs(15),
            base: 2.,
        }
    }
}

/// [`Backoff`] can be created from a [`BackoffConfig`]
///
/// Consecutive calls to [`Backoff::next`] will return the next backoff interval
///
pub(crate) struct Backoff {
    init_backoff: f64,
    next_backoff_secs: f64,
    max_backoff_secs: f64,
    base: f64,
    rng: Option<Box<dyn RngCore + Sync + Send>>,
}

impl std::fmt::Debug for Backoff {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Backoff")
            .field("init_backoff", &self.init_backoff)
            .field("next_backoff_secs", &self.next_backoff_secs)
            .field("max_backoff_secs", &self.max_backoff_secs)
            .field("base", &self.base)
            .finish()
    }
}

impl Backoff {
    /// Create a new [`Backoff`] from the provided [`BackoffConfig`]
    pub(crate) fn new(config: &BackoffConfig) -> Self {
        Self::new_with_rng(config, None)
    }

    /// Creates a new `Backoff` with the optional `rng`
    ///
    /// Used [`rand::rng()`] if no rng provided
    pub(crate) fn new_with_rng(
        config: &BackoffConfig,
        rng: Option<Box<dyn RngCore + Sync + Send>>,
    ) -> Self {
        let init_backoff = config.init_backoff.as_secs_f64();
        Self {
            init_backoff,
            next_backoff_secs: init_backoff,
            max_backoff_secs: config.max_backoff.as_secs_f64(),
            base: config.base,
            rng,
        }
    }

    /// Returns the next backoff duration to wait for
    pub(crate) fn next(&mut self) -> Duration {
        let range = self.init_backoff..(self.next_backoff_secs * self.base);

        let rand_backoff = match self.rng.as_mut() {
            Some(rng) => rng.random_range(range),
            None => rng().random_range(range),
        };

        let next_backoff = self.max_backoff_secs.min(rand_backoff);
        Duration::from_secs_f64(std::mem::replace(&mut self.next_backoff_secs, next_backoff))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::mock::StepRng;

    #[test]
    fn test_backoff() {
        let init_backoff_secs = 1.;
        let max_backoff_secs = 500.;
        let base = 3.;

        let config = BackoffConfig {
            init_backoff: Duration::from_secs_f64(init_backoff_secs),
            max_backoff: Duration::from_secs_f64(max_backoff_secs),
            base,
        };

        let assert_fuzzy_eq = |a: f64, b: f64| assert!((b - a).abs() < 0.0001, "{a} != {b}");

        // Create a static rng that takes the minimum of the range
        let rng = Box::new(StepRng::new(0, 0));
        let mut backoff = Backoff::new_with_rng(&config, Some(rng));

        for _ in 0..20 {
            assert_eq!(backoff.next().as_secs_f64(), init_backoff_secs);
        }

        // Create a static rng that takes the maximum of the range
        let rng = Box::new(StepRng::new(u64::MAX, 0));
        let mut backoff = Backoff::new_with_rng(&config, Some(rng));

        for i in 0..20 {
            let value = (base.powi(i) * init_backoff_secs).min(max_backoff_secs);
            assert_fuzzy_eq(backoff.next().as_secs_f64(), value);
        }

        // Create a static rng that takes the mid point of the range
        let rng = Box::new(StepRng::new(u64::MAX / 2, 0));
        let mut backoff = Backoff::new_with_rng(&config, Some(rng));

        let mut value = init_backoff_secs;
        for _ in 0..20 {
            assert_fuzzy_eq(backoff.next().as_secs_f64(), value);
            value =
                (init_backoff_secs + (value * base - init_backoff_secs) / 2.).min(max_backoff_secs);
        }
    }
}
