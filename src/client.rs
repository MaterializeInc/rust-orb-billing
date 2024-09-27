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

use async_stream::try_stream;
use futures_core::Stream;
use reqwest::{Method, Url};
use reqwest_middleware::{ClientWithMiddleware, RequestBuilder};
use serde::de::DeserializeOwned;
use serde::Deserialize;

use crate::config::ListParams;
use crate::error::ApiError;
use crate::{ClientBuilder, ClientConfig, Error};

pub mod customers;
pub mod events;
pub mod invoices;
pub mod marketplaces;
pub mod plans;
pub mod subscriptions;
pub mod taxes;

/// An API client for Orb.
///
/// The API client is designed to be wrapped in an [`Arc`] and used from
/// multiple threads simultaneously.
///
/// [`Arc`]: std::sync::Arc
#[derive(Debug)]
pub struct Client {
    pub(crate) inner: ClientWithMiddleware,
    pub(crate) api_key: String,
    pub(crate) endpoint: Url,
}

impl Client {
    /// Creates a new `Client` from its required configuration parameters.
    pub fn new(config: ClientConfig) -> Client {
        ClientBuilder::default().build(config)
    }

    /// Creates a builder for a `Client` that allows for customization of
    /// optional parameters.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::default()
    }

    fn build_request<P>(&self, method: Method, path: P) -> RequestBuilder
    where
        P: IntoIterator,
        P::Item: AsRef<str>,
    {
        let mut url = self.endpoint.clone();
        url.path_segments_mut()
            .expect("builder validated URL can be a base")
            .extend(path);
        // All request methods and paths are included to support retries for
        // 429 status code.
        self.inner.request(method, url).bearer_auth(&self.api_key)
    }

    async fn send_request<T>(&self, req: RequestBuilder) -> Result<T, Error>
    where
        T: DeserializeOwned,
    {
        #[derive(Deserialize)]
        struct ErrorResponse {
            title: String,
            #[serde(default)]
            detail: Option<String>,
            #[serde(default)]
            validation_errors: Vec<String>,
        }

        let res = req.send().await?;
        let status_code = res.status();
        if status_code.is_success() {
            Ok(res.json().await?)
        } else {
            let res_body = res.text().await?;
            match serde_json::from_str::<ErrorResponse>(&res_body) {
                Ok(e) => Err(Error::Api(ApiError {
                    status_code,
                    title: e.title,
                    detail: e.detail,
                    validation_errors: e.validation_errors,
                })),
                Err(e) => {
                    eprintln!("There's been an API error! {e:?} from {res_body:?}");
                    Err(Error::Api(ApiError {
                        status_code,
                        title: "decoding failure".into(),
                        detail: Some("unable to decode API response as JSON".into()),
                        validation_errors: vec![],
                    }))
                }
            }
        }
    }

    fn stream_paginated_request<'a, T>(
        &'a self,
        params: &ListParams,
        req: RequestBuilder,
    ) -> impl Stream<Item = Result<T, Error>> + 'a
    where
        T: DeserializeOwned + 'a,
    {
        #[derive(Deserialize)]
        struct Paginated<T> {
            data: Vec<T>,
            pagination_metadata: PaginationMetadata,
        }

        #[derive(Deserialize)]
        struct PaginationMetadata {
            next_cursor: Option<String>,
        }

        let req = req.query(&[("limit", params.page_size)]);
        try_stream! {
            let mut cursor = None;
            loop {
                let mut current_req = req.try_clone().expect("request is clonable");
                if let Some(cursor) = cursor {
                    current_req = current_req.query(&[("cursor", cursor)]);
                }
                let res: Paginated<T> = self.send_request(current_req).await?;
                for datum in res.data {
                    yield datum;
                }
                match res.pagination_metadata.next_cursor {
                    None => break,
                    Some(next_cursor) => cursor = Some(next_cursor),
                }
            }
        }
    }
}
