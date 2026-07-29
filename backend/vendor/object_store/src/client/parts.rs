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

use crate::multipart::PartId;
use parking_lot::Mutex;

/// An interior mutable collection of upload parts and their corresponding part index
#[derive(Debug, Default)]
pub(crate) struct Parts(Mutex<Vec<(usize, PartId)>>);

impl Parts {
    /// Record the [`PartId`] for a given index
    ///
    /// Note: calling this method multiple times with the same `part_idx`
    /// will result in multiple [`PartId`] in the final output
    pub(crate) fn put(&self, part_idx: usize, id: PartId) {
        self.0.lock().push((part_idx, id))
    }

    /// Produce the final list of [`PartId`] ordered by `part_idx`
    ///
    /// `expected` is the number of parts expected in the final result
    pub(crate) fn finish(&self, expected: usize) -> crate::Result<Vec<PartId>> {
        let mut parts = self.0.lock();
        if parts.len() != expected {
            return Err(crate::Error::Generic {
                store: "Parts",
                source: "Missing part".to_string().into(),
            });
        }
        parts.sort_unstable_by_key(|(idx, _)| *idx);
        Ok(parts.drain(..).map(|(_, v)| v).collect())
    }
}
