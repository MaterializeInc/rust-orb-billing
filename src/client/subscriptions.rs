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
use futures_util::stream::TryStreamExt;
use ordered_float::OrderedFloat;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::OffsetDateTime;

use crate::client::customers::{Customer, CustomerId, CustomerResponse};
use crate::client::marketplaces::ExternalMarketplace;
use crate::client::plans::{Plan, PlanId};
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const SUBSCRIPTIONS_PATH: [&str; 1] = ["subscriptions"];

/// An Orb subscription.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CreateSubscriptionRequest<'a> {
    /// An optional user-defined ID for this customer resource, used throughout
    /// the system as an alias for this customer.
    #[serde(flatten)]
    pub customer_id: CustomerId<'a>,
    /// The plan that the customer should be subscribed to.
    ///
    /// The plan determines the pricing and the cadence of the subscription.
    #[serde(flatten)]
    pub plan_id: PlanId<'a>,
    /// The date at which Orb should start billing for the subscription,
    /// localized ot the customer's timezone.
    ///
    /// If `None`, defaults to the current date in the customer's timezone.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub start_date: Option<OffsetDateTime>,
    /// The name of the external marketplace that the subscription is attached
    /// to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub external_marketplace: Option<SubscriptionExternalMarketplaceRequest<'a>>,
    /// Whether to align billing periods with the subscription's start date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub align_billing_with_subscription_start_date: Option<bool>,
    /// The subscription's override minimum amount for the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum_amount: Option<&'a str>,
    /// The subscription's override minimum amount for the plan.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub net_terms: Option<i64>,
    /// Determines whether issued invoices for this subscription will
    /// automatically be charged with the saved payment method on the due date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_collection: Option<bool>,
    /// Determines the default memo on this subscription's invoices.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_invoice_memo: Option<&'a str>,
    /// An idempotency key can ensure that if the same request comes in
    /// multiple times in a 48-hour period, only one makes changes.
    // NOTE: this is passed in a request header, not the body
    #[serde(skip_serializing)]
    pub idempotency_key: Option<&'a str>,
}

/// https://docs.withorb.com/reference/schedule-plan-change
/// This endpoint can be used to change the plan on an existing subscription.
/// The body parameter change_option determines the timing of the plan change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SchedulePlanChangeRequestBody<'a> {
    /// The plan that the customer should be switched to.
    ///
    /// The plan determines the pricing and the cadence of the subscription.
    #[serde(flatten)]
    pub plan_id: PlanId<'a>,
    /// The date that the plan change should take effect.
    /// This parameter can only be passed if the change_option is requested_date.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(with = "time::serde::rfc3339::option")]
    pub change_date: Option<OffsetDateTime>,
    /// Determines the timing of a subscription plan change.
    pub change_option: SubscriptionPlanChangeOption,
    /// When this subscription's accrued usage reaches this threshold, an invoice will be issued
    /// for the subscription. If not specified, invoices will only be issued at the end of the
    /// billing period.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoicing_threshold: Option<&'a str>,
    /// The phase of the plan to start with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_phase_order: Option<i32>,
    /// An idempotency key can ensure that if the same request comes in
    /// multiple times in a 48-hour period, only one makes changes.
    // NOTE: this is passed in a request header, not the body
    #[serde(skip_serializing)]
    pub idempotency_key: Option<&'a str>,
}

/// The body parameter change_option determines the timing of a subscription plan change.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum SubscriptionPlanChangeOption {
    /// requested_date: changes the plan on the requested date (change_date).
    /// If no timezone is provided, the customer's timezone is used.
    /// The change_date body parameter is required if this option is chosen.
    RequestedDate,
    /// end_of_subscription_term: changes the plan at the end of the existing plan's term.
    /// Issuing this plan change request for a monthly subscription will keep the existing
    /// plan active until the start of the subsequent month, and potentially issue an invoice
    /// for any usage charges incurred in the intervening period.
    /// Issuing this plan change request for a yearly subscription will keep the existing plan
    /// active for the full year.
    EndOfSubscriptionTerm,
    /// immediate: changes the plan immediately. Subscriptions that have their plan changed with
    /// this option will be invoiced immediately. This invoice will include any usage fees incurred
    /// in the billing period up to the change, along with any prorated recurring fees for the
    /// billing period, if applicable.
    Immediate,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SubscriptionExternalMarketplaceRequest<'a> {
    /// The kind of the external marketplace.
    #[serde(rename = "external_marketplace")]
    pub kind: ExternalMarketplace,
    /// The ID of the subscription in the external marketplace.
    #[serde(rename = "external_marketplace_reporting_id")]
    pub reporting_id: &'a str,
}

/// An Orb subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Subscription<C = Customer> {
    /// The Orb-assigned unique identifier for the subscription.
    pub id: String,
    /// The customer associated with this subscription.
    pub customer: C,
    /// The plan associated with this subscription.
    pub plan: Plan,
    /// The date at which Orb starts billing for this subscription.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The date at which Orb stops billing for this subscription.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The status of the subscription.
    pub status: Option<SubscriptionStatus>,
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
    /// List of all fixed fee quantities associated with this subscription.
    pub fixed_fee_quantity_schedule: Vec<SubscriptionFixedFee>,
    /// Determines the difference between the invoice issue date and the
    /// date that they are due.
    ///
    /// A value of zero indicates that the invoice is due on issue, whereas a
    /// value of 30 represents that the customer has a month to pay the invoice.
    pub net_terms: i64,
    /// Determines whether issued invoices for this subscription will
    /// automatically be charged with the saved payment method on the due date.
    ///
    /// If `None`, the value is determined by the plan configuration.
    pub auto_collection: bool,
    /// Determines the default memo on this subscription's invoices.
    ///
    /// If `None`, the value is determined by the plan configuration.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_invoice_memo: Option<String>,
    /// The time at which the subscription was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
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

/// An entry in [`Subscription::fixed_fee_quantity_schedule`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct SubscriptionFixedFee {
    /// The date at which the fixed fee starts.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The date at which the fixed fee ends.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
    /// The price ID for the fixed fee.
    pub price_id: String,
    /// The quantity of the fixed fee.
    pub quantity: OrderedFloat<f64>,
}

/// Parameters for a subscription list operation.
#[derive(Debug, Clone)]
pub struct SubscriptionListParams<'a> {
    inner: ListParams,
    filter: Option<CustomerId<'a>>,
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
    pub const fn customer_id(mut self, filter: CustomerId<'a>) -> Self {
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
        let req = self.build_request(Method::GET, SUBSCRIPTIONS_PATH);
        let req = match params.filter {
            None => req,
            Some(CustomerId::Orb(id)) => req.query(&[("customer_id", id)]),
            Some(CustomerId::External(id)) => req.query(&[("external_customer_id", id)]),
        };
        self.stream_paginated_request(&params.inner, req)
            .try_filter_map(|subscription: Subscription<CustomerResponse>| async move {
                match subscription.customer {
                    CustomerResponse::Normal(customer) => Ok(Some(Subscription {
                        id: subscription.id,
                        customer,
                        plan: subscription.plan,
                        start_date: subscription.start_date,
                        end_date: subscription.end_date,
                        status: subscription.status,
                        current_billing_period_start_date: subscription
                            .current_billing_period_start_date,
                        current_billing_period_end_date: subscription
                            .current_billing_period_end_date,
                        active_plan_phase_order: subscription.active_plan_phase_order,
                        fixed_fee_quantity_schedule: subscription.fixed_fee_quantity_schedule,
                        net_terms: subscription.net_terms,
                        auto_collection: subscription.auto_collection,
                        default_invoice_memo: subscription.default_invoice_memo,
                        created_at: subscription.created_at,
                    })),
                    CustomerResponse::Deleted {
                        id: _,
                        deleted: true,
                    } => Ok(None),
                    CustomerResponse::Deleted { id, deleted: false } => {
                        Err(Error::UnexpectedResponse {
                            detail: format!(
                                "customer {id} used deleted response shape \
                                but deleted field was `false`"
                            ),
                        })
                    }
                }
            })
    }

    /// Creates a new subscription.
    pub async fn create_subscription(
        &self,
        subscription: &CreateSubscriptionRequest<'_>,
    ) -> Result<Subscription, Error> {
        let mut req = self.build_request(Method::POST, SUBSCRIPTIONS_PATH);
        if let Some(key) = subscription.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }

        let req = req.json(subscription);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a subscription by ID.
    pub async fn get_subscription(&self, id: &str) -> Result<Subscription, Error> {
        let req = self.build_request(Method::GET, SUBSCRIPTIONS_PATH.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Changes the associated plan for a subscription.
    /// https://docs.withorb.com/reference/schedule-plan-change
    pub async fn schedule_plan_change(
        &self,
        id: &str,
        body: &SchedulePlanChangeRequestBody<'_>,
    ) -> Result<Subscription, Error> {
        let mut req = self.build_request(
            Method::POST,
            SUBSCRIPTIONS_PATH
                .chain_one(id)
                .chain_one("schedule_plan_change"),
        );
        if let Some(key) = body.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        let req = req.json(body);

        let res = self.send_request(req).await?;
        Ok(res)
    }

    // TODO: cancel and unschedule subscriptions.
}
