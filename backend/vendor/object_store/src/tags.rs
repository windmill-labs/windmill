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

use url::form_urlencoded::Serializer;

/// A collection of key value pairs used to annotate objects
///
/// <https://docs.aws.amazon.com/AmazonS3/latest/userguide/object-tagging.html>
/// <https://learn.microsoft.com/en-us/rest/api/storageservices/set-blob-tags>
#[derive(Debug, Clone, Default, Eq, PartialEq)]
pub struct TagSet(String);

impl TagSet {
    /// Append a key value pair to this [`TagSet`]
    ///
    /// Stores have different restrictions on what characters are permitted,
    /// for portability it is recommended applications use no more than 10 tags,
    /// and stick to alphanumeric characters, and `+ - = . _ : /`
    ///
    /// <https://docs.aws.amazon.com/AmazonS3/latest/API/API_PutObjectTagging.html>
    /// <https://learn.microsoft.com/en-us/rest/api/storageservices/set-blob-tags?tabs=azure-ad#request-body>
    pub fn push(&mut self, key: &str, value: &str) {
        Serializer::new(&mut self.0).append_pair(key, value);
    }

    /// Return this [`TagSet`] as a URL-encoded string
    pub fn encoded(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tag_set() {
        let mut set = TagSet::default();
        set.push("test/foo", "value sdlks");
        set.push("foo", " sdf _ /+./sd");
        assert_eq!(
            set.encoded(),
            "test%2Ffoo=value+sdlks&foo=+sdf+_+%2F%2B.%2Fsd"
        );
    }
}
