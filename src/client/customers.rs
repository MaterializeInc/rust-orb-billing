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

use codes_iso_3166::part_1::CountryCode;
use codes_iso_4217::CurrencyCode;
use futures_core::Stream;
use futures_util::stream::TryStreamExt;
use reqwest::Method;
use reqwest_middleware::RequestBuilder;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::format_description::well_known::Rfc3339;
use time::{OffsetDateTime, UtcOffset};

use crate::client::taxes::{TaxId, TaxIdRequest};
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::serde::Empty;
use crate::util::StrIteratorExt;

const CUSTOMERS_PATH: [&str; 1] = ["customers"];

#[derive(Deserialize)]
struct ArrayResponse<T> {
    data: Vec<T>,
}

/// A customer ID.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub enum CustomerId<'a> {
    /// An Orb customer ID.
    #[serde(rename = "customer_id")]
    Orb(&'a str),
    /// A external customer ID.
    #[serde(rename = "external_customer_id")]
    External(&'a str),
}

impl<'a> Default for CustomerId<'a> {
    fn default() -> CustomerId<'a> {
        CustomerId::Orb("")
    }
}

/// The subset of [`Customer`] used in create requests.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CreateCustomerRequest<'a> {
    /// An optional user-defined ID for this customer resource, used throughout
    /// the system as an alias for this customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "external_customer_id")]
    pub external_id: Option<&'a str>,
    /// The full name of the customer.
    pub name: &'a str,
    /// A valid email for the customer, to be used for notifications.
    pub email: &'a str,
    /// Additional email addresses for this customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_emails: Option<Vec<&'a str>>,
    /// The customer's timezone as an identifier from the IANA timezone
    /// database.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timezone: Option<&'a str>,
    /// The external payments or invoicing solution connected to the customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    pub payment_provider: Option<CustomerPaymentProviderRequest<'a>>,
    /// The customer's shipping address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<AddressRequest<'a>>,
    /// The customer's billing address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<AddressRequest<'a>>,
    /// The currency used for the customer's invoices and balance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub currency: Option<CurrencyCode>,
    /// The tax ID details to display on the customer's invoice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<TaxIdRequest<'a>>,
    /// An idempotency key can ensure that if the same request comes in
    /// multiple times in a 48-hour period, only one makes changes.
    // NOTE: this is passed in a request header, not the body
    #[serde(skip_serializing)]
    pub idempotency_key: Option<&'a str>,
}

/// The subset of [`Customer`] used in update requests.
#[derive(Debug, Default, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct UpdateCustomerRequest<'a> {
    /// The full name of the customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<&'a str>,
    /// A valid email for the customer, to be used for notifications.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<&'a str>,
    /// Additional email addresses for this customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_emails: Option<Vec<&'a str>>,
    /// The external payments or invoicing solution connected to the customer.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(flatten)]
    pub payment_provider: Option<CustomerPaymentProviderRequest<'a>>,
    /// The customer's shipping address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shipping_address: Option<AddressRequest<'a>>,
    /// The customer's billing address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub billing_address: Option<AddressRequest<'a>>,
    /// The tax ID details to display on the customer's invoice.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tax_id: Option<TaxIdRequest<'a>>,
}

/// Configures an external payment or invoicing solution for a customer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct CustomerPaymentProviderRequest<'a> {
    /// The external payments or invoicing solution type.
    #[serde(rename = "payment_provider")]
    pub kind: PaymentProvider,
    /// The ID of this customer in an external payments solution, such as
    /// Stripe.
    #[serde(rename = "payment_provider_id")]
    pub id: &'a str,
}

// Deleted variants are immediately filtered out, so boxing the larger
// `Normal` variant would result in an unnecessary heap allocation.
#[allow(clippy::large_enum_variant)]
#[derive(Deserialize)]
#[serde(untagged)]
pub(crate) enum CustomerResponse {
    Normal(Customer),
    Deleted { id: String, deleted: bool },
}

/// An Orb customer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Customer {
    /// The Orb-assigned unique identifier for the customer.
    pub id: String,
    /// An optional user-defined ID for this customer resource, used throughout
    /// the system as an alias for this customer.
    #[serde(rename = "external_customer_id")]
    pub external_id: Option<String>,
    /// The full name of the customer.
    pub name: String,
    /// A valid email for the customer, to be used for notifications.
    pub email: String,
    /// Additional email addresses for this customer.
    pub additional_emails: Vec<String>,
    /// The customer's timezone as an identifier from the IANA timezone
    /// database.
    pub timezone: String,
    /// The ID of this customer in an external payments solution, such as
    /// Stripe.
    pub payment_provider_id: Option<String>,
    /// The external payments or invoicing solution connected to the customer.
    pub payment_provider: Option<PaymentProvider>,
    /// The customer's shipping address.
    pub shipping_address: Option<Address>,
    /// The customer's billing address.
    pub billing_address: Option<Address>,
    /// The currency used for the customer's invoices and balance.
    pub currency: Option<CurrencyCode>,
    /// The tax ID details to display on the customer's invoice.
    pub tax_id: Option<TaxId>,
    /// Undocumented upstream.
    pub auto_collection: bool,
    /// The customer's current balance in their currency.
    pub balance: String,
    /// The time at which the customer was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// An authenticated URL link to the customer's private Orb dashboard portal.
    pub portal_url: Option<String>,
}

/// A payment provider.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
pub enum PaymentProvider {
    /// Quickbooks.
    #[serde(rename = "quickbooks")]
    Quickbooks,
    /// Bill.com.
    #[serde(rename = "bill.com")]
    BillDotCom,
    /// Stripe.
    #[serde(rename = "stripe")]
    Stripe,
    /// Stripe charge.
    #[serde(rename = "stripe_charge")]
    StripeCharge,
    /// Stripe invoice.
    #[serde(rename = "stripe_invoice")]
    StripeInvoice,
    /// Other.
    #[serde(other)]
    Other(String),
}

/// The subset of [`Address`] used in create and update requests.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct AddressRequest<'a> {
    /// The city.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub city: Option<&'a str>,
    /// The country code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub country: Option<CountryCode>,
    /// The first line of the street address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line1: Option<&'a str>,
    /// The second line of the street address.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line2: Option<&'a str>,
    /// The postal code.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub postal_code: Option<&'a str>,
    /// The state.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub state: Option<&'a str>,
}

/// A customer's address.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Address {
    /// The city.
    pub city: Option<String>,
    /// The country code.
    pub country: Option<CountryCode>,
    /// The first line of the street address.
    pub line1: Option<String>,
    /// The second line of the street address.
    pub line2: Option<String>,
    /// The postal code.
    pub postal_code: Option<String>,
    /// The state.
    pub state: Option<String>,
}

/// The types of ledger entries that can be created.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
#[serde(tag = "entry_type")]
pub enum LedgerEntryRequest<'a> {
    /// Increment a credit balance
    #[serde(rename = "increment")]
    Increment(AddIncrementCreditLedgerEntryRequestParams<'a>),
    /// Void an existing ledger entry
    #[serde(rename = "void")]
    Void(AddVoidCreditLedgerEntryRequestParams<'a>),
    // TODO: additional ledger entry types
}

/// Optional invoicing settings for a credit purchase.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize)]
pub struct CreditLedgerInvoiceSettingsRequestParams<'a> {
    /// Whether the credits purchase invoice should auto collect with the customer's saved payment
    /// method.
    pub auto_collection: bool,
    /// The difference between the invoice date and the issue date for the invoice. If due on issue,
    /// set this to `0`.
    pub net_terms: u64,
    /// An optional memo to display on the invoice
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memo: Option<&'a str>,
    /// Whether the credits should be withheld from the customer account until the invoice is paid.
    /// This applies primarily to stripe invoicing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub require_successful_payment: Option<bool>,
}

/// The parameters used to create a customer credit ledger entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct AddIncrementCreditLedgerEntryRequestParams<'a> {
    /// The amount to credit the customer for.
    pub amount: serde_json::Number,
    /// An optional description for the credit operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
    /// The date on which the block's balance will expire.
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expiry_date: Option<OffsetDateTime>,
    /// The date on which the block's balance will become available for use.
    #[serde(with = "time::serde::rfc3339::option")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_date: Option<OffsetDateTime>,
    /// The price per credit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_unit_cost_basis: Option<&'a str>,
    /// Invoicing settings for the credit increment request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub invoice_settings: Option<CreditLedgerInvoiceSettingsRequestParams<'a>>,
}

/// The reason for a void operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
pub enum VoidReason {
    /// The credits are being returned to the originator.
    #[serde(rename = "refund")]
    Refund,
}

/// The parameters used to void a customer credit ledger entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct AddVoidCreditLedgerEntryRequestParams<'a> {
    /// The number of credits to void.
    pub amount: serde_json::Number,
    /// The ID of the credit ledger block to void.
    pub block_id: &'a str,
    /// An optional reason for the void.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub void_reason: Option<VoidReason>,
    /// An optional description for the void operation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<&'a str>,
}

/// A block of credit held by a customer.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct CustomerCreditBlock {
    /// The Orb-assigned unique identifier for the credit block.
    pub id: String,
    /// The remaining credit balance for the block.
    pub balance: serde_json::Number,
    /// The date on which the block's balance will expire.
    #[serde(with = "time::serde::rfc3339::option")]
    pub expiry_date: Option<OffsetDateTime>,
    /// The price per credit.
    pub per_unit_cost_basis: Option<String>,
}

/// The type of ledger entry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "entry_type")]
pub enum LedgerEntry {
    /// Incrementing a credit balance
    #[serde(rename = "increment")]
    Increment(IncrementLedgerEntry),
    /// Voiding of an existing ledger entry
    #[serde(rename = "void")]
    Void(VoidLedgerEntry),
    /// Voiding of an existing ledger entry has been initiated
    #[serde(rename = "void_initiated")]
    VoidInitiated(VoidInitiatedLedgerEntry),
    // TODO: additional ledger entry types
}

/// The state of a ledger entry
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize_enum_str)]
pub enum EntryStatus {
    /// The entry has been committed to the ledger
    #[serde(rename = "committed")]
    Committed,
    /// The entry hasn't yet been committed to the ledger
    #[serde(rename = "pending")]
    Pending,
}

/// A collection of identifiers for a customer
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerIdentifier {
    /// The Orb-assigned unique identifier for the customer.
    pub id: String,
    /// An optional user-defined ID for this customer resource, used throughout
    /// the system as an alias for this customer.
    pub external_customer_id: Option<String>,
}

/// Credit block data associated with entries in a ledger.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LedgerEntryCreditBlock {
    //// The Orb-assigned unique identifier for the credit block.
    pub id: String,
    /// The date on which the block's balance will expire.
    #[serde(with = "time::serde::rfc3339::option")]
    pub expiry_date: Option<OffsetDateTime>,
    /// The price per credit.
    pub per_unit_cost_basis: Option<String>,
}

/// Core ledger entry fields.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BaseLedgerEntry {
    /// The Orb-assigned unique identifier for the ledger entry.
    pub id: String,
    /// An incrementing identifier ordering the ledger entry relative to others.
    pub ledger_sequence_number: u64,
    /// The state of the ledger entry.
    pub entry_status: EntryStatus,
    /// The customer identifiers associated with the ledger entry.
    pub customer: CustomerIdentifier,
    /// The customer's credit balance before application of the ledger operation.
    pub starting_balance: serde_json::Number,
    /// The customer's credit balance after application of the ledger operation.
    pub ending_balance: serde_json::Number,
    /// The amount granted to the ledger.
    pub amount: serde_json::Number,
    /// The date the ledger entry was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// An optional description to associate with the entry.
    pub description: Option<String>,
    /// The credit block the ledger entry is modifying.
    pub credit_block: LedgerEntryCreditBlock,
}

/// A record of an ledger increment operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct IncrementLedgerEntry {
    /// The core ledger entry.
    #[serde(flatten)]
    pub ledger: BaseLedgerEntry,
}

/// A record of a ledger void operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoidLedgerEntry {
    /// The core ledger entry.
    #[serde(flatten)]
    pub ledger: BaseLedgerEntry,
    /// The reason the ledger entry was voided.
    pub void_reason: Option<String>,
    /// The amount voided from the ledger.
    pub void_amount: serde_json::Number,
}

/// A record of a ledger void initialization operation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct VoidInitiatedLedgerEntry {
    /// The core ledger entry.
    #[serde(flatten)]
    pub ledger: BaseLedgerEntry,
    /// The date on which the voided ledger's block will now expire.
    #[serde(with = "time::serde::rfc3339")]
    pub new_block_expiry_date: OffsetDateTime,
    /// The reason the ledger entry was voided.
    pub void_reason: Option<String>,
    /// The amount voided from the ledger.
    pub void_amount: serde_json::Number,
}

/// The view mode for a cost breakdown.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
pub enum CostViewMode {
    /// Provide results as an incremental day-by-day view.
    #[serde(rename = "periodic")]
    Periodic,
    /// Provide results as cumulative totals since the start of the billing period.
    #[serde(rename = "cumulative")]
    Cumulative,
}

#[derive(Debug, Default, Clone)]
struct CustomerCostParamsFilter<'a> {
    timeframe_start: Option<&'a OffsetDateTime>,
    timeframe_end: Option<&'a OffsetDateTime>,
    view_mode: Option<CostViewMode>,
    group_by: Option<&'a str>,
}

trait Filterable<T> {
    /// Apply the filter to a request.
    fn apply(self, filter: &T) -> Self;
}

impl Filterable<CustomerCostParamsFilter<'_>> for RequestBuilder {
    /// Apply the filter to a request.
    fn apply(mut self, filter: &CustomerCostParamsFilter) -> Self {
        if let Some(view_mode) = &filter.view_mode {
            self = self.query(&[("view_mode", view_mode.to_string())]);
        }
        if let Some(group_by) = &filter.group_by {
            self = self.query(&[("group_by", group_by)]);
        }
        if let Some(timeframe_start) = &filter.timeframe_start {
            self = self.query(&[(
                "timeframe_start",
                timeframe_start
                    // Orb requires supplied datetimes be in UTC
                    .to_offset(UtcOffset::UTC)
                    .format(&Rfc3339)
                    .unwrap(),
            )]);
        }
        if let Some(timeframe_end) = &filter.timeframe_end {
            self = self.query(&[(
                "timeframe_end",
                timeframe_end
                    // Orb requires supplied datetimes be in UTC
                    .to_offset(UtcOffset::UTC)
                    .format(&Rfc3339)
                    .unwrap(),
            )]);
        }
        self
    }
}

/// Parameters for a Customer Costs query.
#[derive(Debug, Default, Clone)]
pub struct CustomerCostParams<'a> {
    filter: CustomerCostParamsFilter<'a>,
}

impl<'a> CustomerCostParams<'a> {
    /// The start of the returned range. If not specified this defaults to the billing period start
    /// date.
    pub const fn timeframe_start(mut self, timeframe_start: &'a OffsetDateTime) -> Self {
        self.filter.timeframe_start = Some(timeframe_start);
        self
    }

    /// The end of the returned range. If unspecified will default to the billing period end date.
    pub const fn timeframe_end(mut self, timeframe_end: &'a OffsetDateTime) -> Self {
        self.filter.timeframe_end = Some(timeframe_end);
        self
    }

    /// How costs should be broken down in the resultant day-by-day view.
    pub const fn view_mode(mut self, view_mode: CostViewMode) -> Self {
        self.filter.view_mode = Some(view_mode);
        self
    }

    /// The custom attribute to group costs by.
    pub const fn group_by(mut self, group_by: &'a str) -> Self {
        self.filter.group_by = Some(group_by);
        self
    }
}

/// A group of costs for a given timeframe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostBucket {
    /// Total costs for the timeframe, excluding any minimums and discounts.
    pub subtotal: String,
    /// Total costs for the timeframe, including any minimums and discounts.
    pub total: String,
    /// The starting point for the timeframe.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_start: OffsetDateTime,
    /// The ending point for the timeframe.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_end: OffsetDateTime,
    /// The costs for each price.
    pub per_price_costs: Vec<CustomerCostPriceBlock>,
}

/// The cost for a given Price within a timeframe.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlock {
    /// The price's quantity for the timeframe.
    pub quantity: Option<serde_json::Number>,
    /// The price's contributions for the timeframe, excluding any minimums and discounts.
    pub subtotal: String,
    /// The price's contributions for the timeframe, including any minimums and discounts.
    pub total: String,
    /// The price that can be billed on a subscription.
    pub price: CustomerCostPriceBlockPrice,
    /// The price costs per grouping key.
    pub price_groups: Option<Vec<CustomerCostPriceBlockPriceGroup>>,
}

/// A price cost for a given set of grouping keys.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockPriceGroup {
    /// The key breaking down a single price's costs.
    pub grouping_key: String,
    /// An optional value for the key.
    pub grouping_value: Option<String>,
    /// The second dimension for the matrix price, if applicable.
    pub secondary_grouping_key: Option<String>,
    /// An optional value for the `secondary_grouping_key`, if applicable.
    pub secondary_grouping_value: Option<String>,
    /// Total costs for this group for the timeframe, excluding any minimums and discounts.
    // this should be thought of as a "subtotal" to align with the rest of the API, but we're
    // keeping the existing Orb terminology.
    pub total: String,
}

/// The type of pricing
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(tag = "model_type")]
pub enum CustomerCostPriceBlockPrice {
    /// Sets of unit prices in a one or two-dimensional matrix.
    #[serde(rename = "matrix")]
    Matrix(CustomerCostPriceBlockMatrixPrice),
    /// A fixed amount per unit of usage.
    #[serde(rename = "unit")]
    Unit(CustomerCostPriceBlockUnitPrice),
    // TODO: Add support for additional pricing models
}

/// Matrix pricing details.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockMatrixPrice {
    /// The Orb-assigned unique identifier for the price block.
    pub id: String,
    /// An optional user-defined ID for this price resource.
    #[serde(rename = "external_price_id")]
    pub external_id: Option<String>,
    /// Information about the item being priced.
    pub item: CustomerCostItem,
    /// The configuration for this matrix price.
    pub matrix_config: CustomerCostPriceBlockMatrixPriceConfig,
}

/// An item being priced.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostItem {
    /// Orb's unique identifier for the item.
    pub id: String,
    /// The item's name.
    pub name: String,
}

/// Configuration for a pricing matrix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockMatrixPriceConfig {
    /// The fallback unit amount.
    pub default_unit_amount: String,
    /// A collection of dimensions modeled by the matrix.
    pub dimensions: Vec<Option<String>>,
    /// All pricing values configured for the matrix.
    pub matrix_values: Vec<CustomerCostPriceBlockMatrixPriceValue>,
}

/// A pricing value for a cell within the pricing matrix.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockMatrixPriceValue {
    /// The dimensions corresponding to this cell.
    pub dimension_values: Vec<Option<String>>,
    /// The per-unit amount usage within this cell bills.
    pub unit_amount: String,
}

/// Unit pricing details.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockUnitPrice {
    /// The Orb-assigned unique identifier for the price block.
    pub id: String,
    /// An optional user-defined ID for this price resource.
    #[serde(rename = "external_price_id")]
    pub external_id: Option<String>,
    /// Information about the item being priced.
    pub item: CustomerCostItem,
    /// The configuration for this unit price.
    pub unit_config: CustomerCostPriceBlockUnitPriceConfig,
}

/// Configuration for a unit price.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CustomerCostPriceBlockUnitPriceConfig {
    /// Per-unit pricing.
    pub unit_amount: String,
}

impl Client {
    /// Lists all customers.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_customers(
        &self,
        params: &ListParams,
    ) -> impl Stream<Item = Result<Customer, Error>> + '_ {
        let req = self.build_request(Method::GET, CUSTOMERS_PATH);
        self.stream_paginated_request(params, req)
            .try_filter_map(|res| async {
                match res {
                    CustomerResponse::Normal(c) => Ok(Some(c)),
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

    /// Creates a new customer.
    pub async fn create_customer(
        &self,
        customer: &CreateCustomerRequest<'_>,
    ) -> Result<Customer, Error> {
        let mut req = self.build_request(Method::POST, CUSTOMERS_PATH);
        if let Some(key) = customer.idempotency_key {
            req = req.header("Idempotency-Key", key);
        }
        let req = req.json(customer);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a customer by ID.
    pub async fn get_customer(&self, id: &str) -> Result<Customer, Error> {
        let req = self.build_request(Method::GET, CUSTOMERS_PATH.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Gets a customer by external ID.
    pub async fn get_customer_by_external_id(&self, external_id: &str) -> Result<Customer, Error> {
        let req = self.build_request(
            Method::GET,
            CUSTOMERS_PATH
                .chain_one("external_customer_id")
                .chain_one(external_id),
        );
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Updates a customer by ID.
    pub async fn update_customer(
        &self,
        id: &str,
        customer: &UpdateCustomerRequest<'_>,
    ) -> Result<Customer, Error> {
        let req = self.build_request(Method::PUT, CUSTOMERS_PATH.chain_one(id));
        let req = req.json(customer);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Updates a customer by external ID.
    pub async fn update_customer_by_external_id(
        &self,
        external_id: &str,
        customer: &UpdateCustomerRequest<'_>,
    ) -> Result<Customer, Error> {
        let req = self.build_request(
            Method::PUT,
            CUSTOMERS_PATH
                .chain_one("external_customer_id")
                .chain_one(external_id),
        );
        let req = req.json(customer);
        let res = self.send_request(req).await?;
        Ok(res)
    }

    /// Deletes a customer by ID.
    pub async fn delete_customer(&self, id: &str) -> Result<(), Error> {
        let req = self.build_request(Method::DELETE, CUSTOMERS_PATH.chain_one(id));
        let _: Empty = self.send_request(req).await?;
        Ok(())
    }

    /// Fetch all unexpired, non-zero credit blocks for a customer.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn get_customer_credit_balance(
        &self,
        id: &str,
        params: &ListParams,
    ) -> impl Stream<Item = Result<CustomerCreditBlock, Error>> + '_ {
        let req = self.build_request(
            Method::GET,
            CUSTOMERS_PATH.chain_one(id).chain_one("credits"),
        );
        self.stream_paginated_request(params, req)
    }

    /// Fetch all unexpired, non-zero credit blocks for a customer by external ID.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn get_customer_credit_balance_by_external_id(
        &self,
        external_id: &str,
        params: &ListParams,
    ) -> impl Stream<Item = Result<CustomerCreditBlock, Error>> + '_ {
        let req = self.build_request(
            Method::GET,
            CUSTOMERS_PATH
                .chain_one("external_customer_id")
                .chain_one(external_id)
                .chain_one("credits"),
        );
        self.stream_paginated_request(params, req)
    }

    /// Create a new ledger entry for the specified customer's balance.
    pub async fn create_ledger_entry(
        &self,
        id: &str,
        entry: &LedgerEntryRequest<'_>,
    ) -> Result<LedgerEntry, Error> {
        let req = self.build_request(
            Method::POST,
            CUSTOMERS_PATH
                .chain_one(id)
                .chain_one("credits")
                .chain_one("ledger_entry"),
        );
        let req = req.json(entry);
        self.send_request(req).await
    }

    /// Fetch a day-by-day snapshot of a customer's costs.
    pub async fn get_customer_costs(
        &self,
        id: &str,
        params: &CustomerCostParams<'_>,
    ) -> Result<Vec<CustomerCostBucket>, Error> {
        let req = self.build_request(Method::GET, CUSTOMERS_PATH.chain_one(id).chain_one("costs"));
        let req = req.apply(&params.filter);
        let res: ArrayResponse<CustomerCostBucket> = self.send_request(req).await?;
        Ok(res.data)
    }

    /// Fetch a day-by-day snapshot of a customer's costs by their external ID.
    pub async fn get_customer_costs_by_external_id(
        &self,
        external_id: &str,
        params: &CustomerCostParams<'_>,
    ) -> Result<Vec<CustomerCostBucket>, Error> {
        let req = self.build_request(
            Method::GET,
            CUSTOMERS_PATH
                .chain_one("external_customer_id")
                .chain_one(external_id)
                .chain_one("costs"),
        );
        let req = req.apply(&params.filter);
        let res: ArrayResponse<CustomerCostBucket> = self.send_request(req).await?;
        Ok(res.data)
    }
}
