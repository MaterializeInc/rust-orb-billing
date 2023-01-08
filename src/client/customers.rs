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
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::OffsetDateTime;

use crate::client::taxes::{TaxId, TaxIdRequest};
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::serde::Empty;
use crate::util::StrIteratorExt;

const CUSTOMERS_PATH: [&str; 1] = ["customers"];

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
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
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
        let req = self.build_request(Method::POST, CUSTOMERS_PATH);
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
}
