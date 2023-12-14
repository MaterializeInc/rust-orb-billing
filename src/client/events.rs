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

use std::collections::BTreeMap;

use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::Number;
use time::OffsetDateTime;

use crate::client::customers::CustomerId;
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const EVENTS_PATH: [&str; 1] = ["events"];

/// The subset of [`Event`] used in event ingestion requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct IngestEventRequest<'a> {
    /// A unique, client-generated identifier for the event.
    ///
    /// Exactly one event with a given idempotency key will be ingested, which
    /// allows for safe request retries.
    pub idempotency_key: &'a str,
    /// The Orb ID for the customer with which the event is associated.
    #[serde(flatten)]
    pub customer_id: CustomerId<'a>,
    /// A name that meaningfully identifies the action or event.
    pub event_name: &'a str,
    /// Arbitrary properties associated with the event.
    pub properties: &'a BTreeMap<String, EventPropertyValue>,
    /// The time at which the event occurred.
    ///
    /// Important for attributing usage to a given billing period.
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

/// The ingestion mode.
#[derive(Debug, Clone, Copy)]
pub enum IngestionMode {
    /// Enable debugging information.
    Debug,
    /// Disable debugging information for improved performance.
    Production,
}

/// The response to an event ingestion request.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct IngestEventResponse {
    /// Debug information.
    ///
    /// Only present when the request is made with [`IngestionMode::Debug`].
    pub debug: Option<IngestEventDebugResponse>,
}

/// The type of [`IngestEventResponse::debug`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct IngestEventDebugResponse {
    /// Contains the IDs of events that were already known to Orb.
    pub duplicate: Vec<String>,
    /// Contains the IDs of events that were new to Orb.
    pub ingested: Vec<String>,
}

/// The subset of [`Event`] used in amendment requests.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct AmendEventRequest<'a> {
    /// The Orb ID for the customer with which the event is associated.
    #[serde(flatten)]
    pub customer_id: CustomerId<'a>,
    /// A name that meaningfully identifies the action or event.
    pub event_name: &'a str,
    /// Arbitrary properties associated with the event.
    pub properties: &'a BTreeMap<String, EventPropertyValue>,
    /// The time at which the event occurred.
    ///
    /// Important for attributing usage to a given billing period.
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

/// An Orb event.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Event {
    /// A unique, client-generated identifier for the event.
    pub id: String,
    /// The Orb ID for the customer with which the event is associated.
    pub customer_id: String,
    /// The external ID for the customer with which the event is associated.
    pub external_customer_id: Option<String>,
    /// A name that meaningfully identifies the action or event.
    pub event_name: String,
    /// Arbitrary properties associated with the event.
    pub properties: BTreeMap<String, EventPropertyValue>,
    /// The time at which the event occurred.
    ///
    /// Important for attributing usage to a given billing period.
    #[serde(with = "time::serde::rfc3339")]
    pub timestamp: OffsetDateTime,
}

/// The value of a property assocaited with an [`Event`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(untagged)]
pub enum EventPropertyValue {
    /// A string value.
    String(String),
    /// A numeric value.
    Number(Number),
    /// A boolean value.
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
struct EventFilter<'a> {
    #[serde(skip_serializing_if = "Option::is_none")]
    event_ids: Option<&'a [&'a str]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    invoice_id: Option<&'a str>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    timeframe_start: Option<OffsetDateTime>,
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    timeframe_end: Option<OffsetDateTime>,
}

/// Parameters for an event search operation.
#[derive(Debug, Clone)]
pub struct EventSearchParams<'a> {
    inner: ListParams,
    filter: EventFilter<'a>,
}

impl<'a> Default for EventSearchParams<'a> {
    fn default() -> EventSearchParams<'a> {
        EventSearchParams::DEFAULT
    }
}

impl<'a> EventSearchParams<'a> {
    /// The default subscription list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: EventSearchParams<'static> = EventSearchParams {
        inner: ListParams::DEFAULT,
        filter: EventFilter {
            event_ids: None,
            invoice_id: None,
            timeframe_start: None,
            timeframe_end: None,
        },
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }

    /// Filters the search to the specified event IDs.
    pub const fn event_ids(mut self, filter: &'a [&'a str]) -> Self {
        self.filter.event_ids = Some(filter);
        self
    }

    /// Filters the search to the specified invoice ID.
    pub const fn invoice_id(mut self, filter: &'a str) -> Self {
        self.filter.invoice_id = Some(filter);
        self
    }

    /// Filters the search to events falling on or after the specified datetime.
    pub const fn timeframe_start(mut self, start: OffsetDateTime) -> Self {
        self.filter.timeframe_start = Some(start);
        self
    }

    /// Filters the search to events falling before the specified datetime.
    pub const fn timeframe_end(mut self, end: OffsetDateTime) -> Self {
        self.filter.timeframe_end = Some(end);
        self
    }
}

impl Client {
    /// Searches events.
    pub fn search_events(
        &self,
        params: &EventSearchParams,
    ) -> impl Stream<Item = Result<Event, Error>> + '_ {
        let req = self.build_request(Method::POST, EVENTS_PATH.chain_one("search"));
        let req = req.json(&params.filter);
        self.stream_paginated_request(&params.inner, req)
    }

    /// Ingests events.
    pub async fn ingest_events(
        &self,
        mode: IngestionMode,
        events: &[IngestEventRequest<'_>],
    ) -> Result<IngestEventResponse, Error> {
        #[derive(Serialize)]
        struct Envelope<'a> {
            events: &'a [IngestEventRequest<'a>],
        }

        let req = self.build_request(Method::POST, ["ingest"]);
        let req = req.query(&[("debug", matches!(mode, IngestionMode::Debug))]);
        let req = req.json(&Envelope { events });
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Amends an event by ID.
    pub async fn amend_event(&self, id: &str, event: &AmendEventRequest<'_>) -> Result<(), Error> {
        let req = self.build_request(Method::PUT, EVENTS_PATH.chain_one(id));
        let req = req.json(event);
        let _: serde_json::Value = self.send_request(req).await?;
        Ok(())
    }

    /// Deprecates an event by ID.
    pub async fn deprecate_event(&self, id: &str) -> Result<(), Error> {
        let req = self.build_request(
            Method::PUT,
            EVENTS_PATH.chain_one(id).chain_one("deprecate"),
        );
        let _: serde_json::Value = self.send_request(req).await?;
        Ok(())
    }
}
