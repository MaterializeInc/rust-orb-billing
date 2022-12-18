// Copyright Materialize, Inc. All rights reserved.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License in the LICENSE file at the
// root of this repository, or online at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::time::Duration;

use once_cell::sync::Lazy;
use reqwest::Url;

use crate::Client;

pub static DEFAULT_ENDPOINT: Lazy<Url> = Lazy::new(|| {
    "https://api.billwithorb.com/v1"
        .parse()
        .expect("url known to be valid")
});

/// Configures the required parameters of a [`Client`].
pub struct ClientConfig {
    /// The API key to authenticate with.
    pub api_key: String,
}

/// A builder for a [`Client`].
pub struct ClientBuilder {
    endpoint: Url,
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        ClientBuilder {
            endpoint: DEFAULT_ENDPOINT.clone(),
        }
    }
}

impl ClientBuilder {
    /// Creates a [`Client`] that incorporates the optional parameters
    /// configured on the builder and the specified required parameters.
    pub fn build(self, config: ClientConfig) -> Client {
        let inner = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Client {
            inner,
            api_key: config.api_key,
            endpoint: self.endpoint,
        }
    }
}

/// Parameters for a list operation.
#[derive(Debug, Clone)]
pub struct ListParams {
    pub(crate) page_size: u64,
}

impl Default for ListParams {
    fn default() -> ListParams {
        ListParams::DEFAULT
    }
}

impl ListParams {
    /// The default list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: ListParams = ListParams { page_size: 20 };

    /// Sets the page size for the list operation.
    ///
    /// The page size only affects the size of each HTTP response. It does not
    /// change the observable output of the API.
    ///
    /// The default page size is 20. The maximum page size is 500.
    ///
    /// See the [Orb API pagination documentation][orb-docs] for details.
    ///
    /// [orb-docs]: https://docs.withorb.com/docs/orb-docs/05ccf93502e54-pagination
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.page_size = page_size;
        self
    }
}
