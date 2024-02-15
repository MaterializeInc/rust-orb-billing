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
use time::OffsetDateTime;

use crate::Price;
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const PLANS_PATH: [&str; 1] = ["plans"];

/// A plan ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum PlanId<'a> {
    /// An Orb plan ID.
    #[serde(rename = "plan_id")]
    Orb(&'a str),
    /// An external plan ID.
    #[serde(rename = "external_plan_id")]
    External(&'a str),
}

impl<'a> Default for PlanId<'a> {
    fn default() -> PlanId<'a> {
        PlanId::Orb("")
    }
}

/// An Orb plan.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Plan {
    /// The Orb-assigned unique identifier for the plan.
    pub id: String,
    /// An optional user-defined ID for this plan resource, used throughout
    /// the system as an alias for this plan.
    #[serde(rename = "external_plan_id")]
    pub external_id: Option<String>,
    /// A human-readable name for the plan.
    pub name: Option<String>,
    /// A human-readable description of the plan.
    pub description: String,
    /// The time at which the plan was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The parent plan id if the given plan was created by overriding one or more of the parent's
    /// prices.
    pub base_plan_id: Option<String>,
    /// Arbitrary metadata that is attached to the plan. Cannot be nested, must have string values.
    #[serde(default)]
    pub metadata: BTreeMap<String, String>,
    /// One of active, archived, draft
    pub status: String,
    /// Prices for this plan
    pub prices: Vec<Price>,
    // TODO: many missing fields.
}

/// Parameters for a plan list operation.
#[derive(Debug, Clone)]
pub struct PlanListParams<'a> {
    inner: ListParams,
    status_filter: Option<&'a str>,
}

impl<'a> Default for PlanListParams<'a> {
    fn default() -> PlanListParams<'a> {
        PlanListParams::DEFAULT
    }
}

impl<'a> PlanListParams<'a> {
    /// The default plan list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: PlanListParams<'static> = PlanListParams {
        inner: ListParams::DEFAULT,
        status_filter: None,
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }

    /// Filters the listing by status
    pub const fn status(mut self, filter: &'a str) -> Self {
        self.status_filter = Some(filter);
        self
    }
}

impl Client {
    /// Lists all plans.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_plans(&self, params: &PlanListParams) -> impl Stream<Item = Result<Plan, Error>> + '_ {
        let req = self.build_request(Method::GET, PLANS_PATH);
        let req = match params.status_filter {
            None => req,
            Some(status) => req.query(&[("status", status)]),
        };
        self.stream_paginated_request(&params.inner, req)
    }

    /// Gets a plan by ID.
    pub async fn get_plan(&self, id: &str) -> Result<Plan, Error> {
        let req = self.build_request(Method::GET, PLANS_PATH.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a plan by external ID.
    pub async fn get_plan_by_external_id(&self, external_id: &str) -> Result<Plan, Error> {
        let req = self.build_request(
            Method::GET,
            PLANS_PATH
                .chain_one("external_plan_id")
                .chain_one(external_id),
        );
        let res = self.send_request(req).await?;
        Ok(res)
    }
}
