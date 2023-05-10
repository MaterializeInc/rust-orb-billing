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
use serde::de::Unexpected;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use time::OffsetDateTime;

use crate::client::customers::CustomerId;
use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;
use crate::util::StrIteratorExt;

const INVOICES: [&str; 1] = ["invoices"];

/// An ISO 4217 currency string, or "credits".
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Currency {
    Credits,
    Iso4217(String),
}

impl<'de> Deserialize<'de> for Currency {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let mut s = String::deserialize(d)?;
        if s == "credits" {
            Ok(Self::Credits)
        } else if s.len() == 3 {
            s.make_ascii_uppercase();
            Ok(Self::Iso4217(s))
        } else {
            use serde::de::Error;
            Err(D::Error::invalid_value(
                Unexpected::Str(&s),
                &"either \"credits\", or a three-character currency code",
            ))
        }
    }
}

impl Serialize for Currency {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = match self {
            Currency::Credits => "credits",
            Currency::Iso4217(code) => code,
        };
        s.serialize(serializer)
    }
}

/// An Orb invoice.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Invoice {
    /// The Orb-assigned unique identifier for the invoice.
    pub id: String,
    /// The customer to whom this invoice was issued.
    pub customer: InvoiceCustomer,
    /// The subscription associated with this invoice.
    pub subscription: InvoiceSubscription,
    /// The issue date of the invoice.
    #[serde(with = "time::serde::rfc3339")]
    pub invoice_date: OffsetDateTime,
    /// The link to download the PDF representation of the invoice.
    pub invoice_pdf: Option<String>,
    /// The total after any minimums, discounts, and taxes have been applied.
    pub total: String,
    /// The time at which the invoice was created.
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The link to the hosted invoice.
    pub hosted_invoice_url: Option<String>,
    /// The status (see [`InvoiceStatusFilter`] for details).
    pub status: String,
    /// The currency in which the invoice is denominated.
    pub currency: Currency,
    // TODO: many missing fields.
}

/// Identifies the customer associated with an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct InvoiceCustomer {
    /// The Orb-assigned unique identifier for the customer.
    pub id: String,
    /// The external identifier for the customer, if any.
    #[serde(rename = "external_customer_id")]
    pub external_id: Option<String>,
}

/// Identifies the subscription associated with an [`Invoice`].
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct InvoiceSubscription {
    /// The Orb-assigned unique identifier for the subscription.
    pub id: String,
}

/// Identifies the statuses of which [`Invoice`]s should be returned.
#[derive(Debug, Clone, Copy)]
pub struct InvoiceStatusFilter {
    /// Draft -- invoices in their initial state.
    pub draft: bool,
    /// Issued -- invoices after their billing period ends.
    pub issued: bool,
    /// Paid -- invoices upon confirmation of successful automatic
    /// payment collection.
    pub paid: bool,
    /// Void -- invoices that have been manually voided.
    pub void: bool,
    /// Synced -- invoices that have been synced to an external
    /// billing provider.
    pub synced: bool,
}

impl Default for InvoiceStatusFilter {
    fn default() -> Self {
        Self::DEFAULT
    }
}

impl InvoiceStatusFilter {
    /// The default invoice list status filter.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: InvoiceStatusFilter = InvoiceStatusFilter {
        issued: true,
        paid: true,
        synced: true,
        draft: false,
        void: false,
    };
}

/// Parameters for a subscription list operation.
#[derive(Debug, Clone)]
pub struct InvoiceListParams<'a> {
    inner: ListParams,
    customer_filter: Option<CustomerId<'a>>,
    subscription_filter: Option<&'a str>,
    status_filter: InvoiceStatusFilter,
}

impl<'a> Default for InvoiceListParams<'a> {
    fn default() -> InvoiceListParams<'a> {
        InvoiceListParams::DEFAULT
    }
}

impl<'a> InvoiceListParams<'a> {
    /// The default invoice list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: InvoiceListParams<'static> = InvoiceListParams {
        inner: ListParams::DEFAULT,
        customer_filter: None,
        subscription_filter: None,
        status_filter: InvoiceStatusFilter::DEFAULT,
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
        self.customer_filter = Some(filter);
        self
    }

    /// Filters the listing to the specified subscription ID.
    pub const fn subscription_id(mut self, filter: &'a str) -> Self {
        self.subscription_filter = Some(filter);
        self
    }

    /// Filters the listing to a specified set of statuses.
    pub const fn status_filter(mut self, filter: InvoiceStatusFilter) -> Self {
        self.status_filter = filter;
        self
    }
}

impl Client {
    /// Lists invoices as configured by `params`.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_invoices(
        &self,
        params: &InvoiceListParams,
    ) -> impl Stream<Item = Result<Invoice, Error>> + '_ {
        let req = self.build_request(Method::GET, INVOICES);
        let req = match params.customer_filter {
            None => req,
            Some(CustomerId::Orb(id)) => req.query(&[("customer_id", id)]),
            Some(CustomerId::External(id)) => req.query(&[("external_customer_id", id)]),
        };
        let req = match params.subscription_filter {
            None => req,
            Some(id) => req.query(&[("subscription_id", id)]),
        };
        let InvoiceStatusFilter {
            draft,
            issued,
            paid,
            void,
            synced,
        } = params.status_filter;
        let mut req = req;
        for (name, value) in [
            ("draft", draft),
            ("issued", issued),
            ("paid", paid),
            ("void", void),
            ("synced", synced),
        ] {
            if value {
                req = req.query(&[("status[]", name)])
            }
        }
        self.stream_paginated_request(&params.inner, req)
    }

    /// Gets an invoice by ID.
    pub async fn get_invoice(&self, id: &str) -> Result<Invoice, Error> {
        let req = self.build_request(Method::GET, INVOICES.chain_one(id));
        let res = self.send_request(req).await?;
        Ok(res)
    }

    // TODO: get upcoming invoice.
    // TODO: void invoice.
}
