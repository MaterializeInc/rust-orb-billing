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
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

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
    // TODO: many missing fields.
}

impl Client {
    /// Lists all plans.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_plans(&self, params: &ListParams) -> impl Stream<Item = Result<Plan, Error>> + '_ {
        let req = self.build_request(Method::GET, PLANS_PATH);
        self.stream_paginated_request(params, req)
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
