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

use futures_core::Stream;
use reqwest::Method;
use serde::Deserialize;
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::OffsetDateTime;

use crate::client::customers::{Customer, CustomerIdFilter};
use crate::client::plans::Plan;
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const SUBSCRIPTIONS: [&str; 1] = ["subscriptions"];

/// An Orb subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub struct Subscription {
    /// The Orb-assigned unique identifier for the subscription.
    pub id: String,
    /// The customer associated with this subscription.
    pub customer: Customer,
    /// The plan associated with this subscription.
    pub plan: Plan,
    /// The date at which Orb starts billing for this subscription.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The date at which Orb stops billing for this subscription.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The start of the current billing period if the subscription is currently
    /// active.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_billing_period_start_date: Option<OffsetDateTime>,
    /// The end of the current billing period if the subscription is currently
    /// active.
    #[serde(with = "time::serde::rfc3339::option")]
    pub current_billing_period_end_date: Option<OffsetDateTime>,
    /// The current plan phase that is active, if the subscription's plan has
    /// phases.
    pub active_plan_phase_order: Option<i64>,
    /// The time at which the subscription was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    // TODO: fixed_fee_quantity_schedule.
}

/// The status of an Orb subscription.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionStatus {
    /// An active subscription.
    Active,
    /// A subscription that has ended.
    Ended,
    /// A subscription that has not yet started.
    Upcoming,
    /// An unknown subscription status.
    #[serde(other)]
    Other(String),
}

/// Parameters for a subscription list operation.
#[derive(Debug, Clone)]
pub struct SubscriptionListParams<'a> {
    inner: ListParams,
    filter: Option<CustomerIdFilter<'a>>,
}

impl<'a> Default for SubscriptionListParams<'a> {
    fn default() -> SubscriptionListParams<'a> {
        SubscriptionListParams::DEFAULT
    }
}

impl<'a> SubscriptionListParams<'a> {
    /// The default subscription list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: SubscriptionListParams<'static> = SubscriptionListParams {
        inner: ListParams::DEFAULT,
        filter: None,
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }

    /// Filters the listing to the specified customer ID.
    pub const fn customer_id(mut self, filter: CustomerIdFilter<'a>) -> Self {
        self.filter = Some(filter);
        self
    }
}

impl Client {
    /// Lists subscriptions as configured by `params`.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_subscriptions(
        &self,
        params: &SubscriptionListParams,
    ) -> impl Stream<Item = Result<Subscription, Error>> + '_ {
        let req = self.build_request(Method::GET, SUBSCRIPTIONS);
        let req = match params.filter {
            None => req,
            Some(CustomerIdFilter::Orb(id)) => req.query(&[("customer_id", id)]),
            Some(CustomerIdFilter::External(id)) => req.query(&[("external_customer_id", id)]),
        };
        self.stream_paginated_request(&params.inner, req)
    }

    /// Gets a subscription by ID.
    pub async fn get_subscription(&self, id: &str) -> Result<Subscription, Error> {
        let req = self.build_request(Method::GET, SUBSCRIPTIONS.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    // TODO: create, cancel, and unschedule subscriptions.
}
