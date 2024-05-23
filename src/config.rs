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
use reqwest::{Response, Url};
use reqwest_retry::policies::ExponentialBackoff;
use reqwest_retry::{
    default_on_request_failure, RetryTransientMiddleware, Retryable, RetryableStrategy,
};

use crate::client::Client;

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
    retry_policy: Option<ExponentialBackoff>,
}

impl Default for ClientBuilder {
    fn default() -> ClientBuilder {
        ClientBuilder {
            endpoint: DEFAULT_ENDPOINT.clone(),
            retry_policy: Some(
                ExponentialBackoff::builder()
                    .retry_bounds(Duration::from_secs(1), Duration::from_secs(5))
                    .build_with_max_retries(5),
            ),
        }
    }
}

/// Retry requests with a successful response of 429 (too many requests).
struct Retry429;
impl RetryableStrategy for Retry429 {
    fn handle(&self, res: &Result<Response, reqwest_middleware::Error>) -> Option<Retryable> {
        match res {
            // Retry if response status is 429
            Ok(success) if success.status() == 429 => Some(Retryable::Transient),
            // Otherwise do not retry a successful request
            Ok(_) => None,
            // Retry failures due to network errors
            Err(error) => default_on_request_failure(error),
        }
    }
}

impl ClientBuilder {
    /// Sets the policy for retrying failed API calls.
    ///
    /// Note that the created [`Client`] will retry all API calls that return a 429 status code.
    pub fn with_retry_policy(mut self, policy: ExponentialBackoff) -> Self {
        self.retry_policy = Some(policy);
        self
    }

    /// Sets the endpoint.
    pub fn with_endpoint(mut self, endpoint: Url) -> Self {
        self.endpoint = endpoint;
        self
    }

    /// Creates a [`Client`] that incorporates the optional parameters
    /// configured on the builder and the specified required parameters.
    pub fn build(self, config: ClientConfig) -> Client {
        let client = reqwest::ClientBuilder::new()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap();
        Client {
            client_retryable: match self.retry_policy {
                Some(policy) => reqwest_middleware::ClientBuilder::new(client.clone())
                    .with(RetryTransientMiddleware::new_with_policy_and_strategy(
                        policy, Retry429,
                    ))
                    .build(),
                None => reqwest_middleware::ClientBuilder::new(client.clone()).build(),
            },
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
