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

//! Integration tests.
//!
//! To run these tests, you must create an Orb account and provide an API key in
//! the `ORB_API_KEY` environment variable.
//!
//! These tests must be run serially, as via
//!
//!     $ cargo test -- --test-threads=1
//!
//! because each test competes for access to the same Orb account.

use std::collections::HashSet;
use std::env;
use std::fmt;

use codes_iso_3166::part_1::CountryCode;
use futures::future;
use futures::stream::TryStreamExt;
use once_cell::sync::Lazy;
use rand::Rng;
use reqwest::StatusCode;
use test_log::test;
use tracing::info;

use orb_billing::{
    Address, AddressRequest, Client, ClientConfig, CreateCustomerRequest, Error, ListParams, TaxId,
    TaxIdRequest, UpdateCustomerRequest, SubscriptionListParams, InvoiceListParams,
};

/// The API key to authenticate with.
static API_KEY: Lazy<String> = Lazy::new(|| env::var("ORB_API_KEY").expect("missing ORB_API_KEY"));

/// When performing parallel operations against the Orb API, the maximum
/// number of concurrent operations to run.
const CONCURRENCY_LIMIT: usize = 16;

/// A prefix to use in objects to make it possible to determine whether a given
/// object was created by this test script or not.
///
/// Required because we do not have exclusive access to the Orb account. Other
/// tests may be running against this account, so we do not want to blindly
/// delete all existing objects at the start of the test.
const TEST_PREFIX: &str = "$TEST-RUST-API$";

/// A `ListParams` that uses the maximum possible page size.
const MAX_PAGE_LIST_PARAMS: ListParams = ListParams::DEFAULT.page_size(500);

fn new_client() -> Client {
    Client::new(ClientConfig {
        api_key: API_KEY.clone(),
    })
}

async fn delete_all_test_customers(client: &Client) {
    client
        .list_customers(&MAX_PAGE_LIST_PARAMS)
        .try_filter(|customer| future::ready(customer.name.starts_with(TEST_PREFIX)))
        .try_for_each_concurrent(Some(CONCURRENCY_LIMIT), |customer| async move {
            info!(%customer.id, "deleting custome");
            client.delete_customer(&customer.id).await
        })
        .await
        .unwrap()
}

fn assert_error_with_status_code<T>(res: Result<T, Error>, status_code: StatusCode)
where
    T: fmt::Debug,
{
    match res.unwrap_err() {
        Error::Transport(e) => {
            panic!("expected API error with code {status_code} but got transport error: {e}")
        }
        Error::Api(e) => assert_eq!(e.status_code, status_code),
    }
}

#[test(tokio::test)]
async fn test_customers() {
    // Set up.
    let client = new_client();
    let nonce = rand::thread_rng().gen::<u32>();
    delete_all_test_customers(&client).await;

    // Test creating a customer.
    let name = format!("{TEST_PREFIX}-{nonce}");
    let email = "orb-testing@materialize.com";
    let external_id = format!("{TEST_PREFIX}-{nonce}");
    let customer = client
        .create_customer(&CreateCustomerRequest {
            name: &*name,
            email,
            external_id: Some(&*external_id),
            timezone: Some("America/New_York"),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);
    assert_eq!(customer.external_id.as_ref(), Some(&external_id));
    assert_eq!(customer.timezone, "America/New_York");
    assert_eq!(customer.balance, "0.00");
    assert_eq!(customer.billing_address, None);
    assert_eq!(customer.shipping_address, None);
    assert_eq!(customer.tax_id, None);

    // Test fetching the customer by ID.
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);

    // Test fetching the customer by external ID.
    let customer = client
        .get_customer_by_external_id(&*external_id)
        .await
        .unwrap();
    assert_eq!(customer.name, name);
    assert_eq!(customer.email, email);

    // Test updating the customer by ID.
    let customer = client
        .update_customer(
            &customer.id,
            &UpdateCustomerRequest {
                email: Some("orb-testing+update-1@materialize.com"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(customer.email, "orb-testing+update-1@materialize.com");
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.email, "orb-testing+update-1@materialize.com");

    // Test updating the customer by external ID.
    let customer = client
        .update_customer_by_external_id(
            &external_id,
            &UpdateCustomerRequest {
                email: Some("orb-testing+update-2@materialize.com"),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(customer.email, "orb-testing+update-2@materialize.com");
    let customer = client.get_customer(&customer.id).await.unwrap();
    assert_eq!(customer.email, "orb-testing+update-2@materialize.com");

    // Test creating a second customer, and exercise addresses and tax IDs.
    let email2 = "orb-testing+2@materialize.com";
    let customer2 = client
        .create_customer(&CreateCustomerRequest {
            name: &*format!("{TEST_PREFIX}-{nonce}-2"),
            email: email2,
            shipping_address: Some(AddressRequest {
                city: Some("New York"),
                country: Some(CountryCode::US),
                line1: Some("440 Lafayette St"),
                line2: Some("Floor 6"),
                postal_code: Some("10003"),
                state: Some("NY"),
            }),
            billing_address: Some(AddressRequest {
                city: Some("Boston"),
                country: Some(CountryCode::US),
                ..Default::default()
            }),
            tax_id: Some(TaxIdRequest {
                type_: orb_billing::TaxIdType::UsEin,
                value: "12-3456789",
                country: CountryCode::US,
            }),
            ..Default::default()
        })
        .await
        .unwrap();
    assert_eq!(
        customer2.shipping_address,
        Some(Address {
            city: Some("New York".into()),
            country: Some(CountryCode::US),
            line1: Some("440 Lafayette St".into()),
            line2: Some("Floor 6".into()),
            postal_code: Some("10003".into()),
            state: Some("NY".into()),
        })
    );
    assert_eq!(
        customer2.billing_address,
        Some(Address {
            city: Some("Boston".into()),
            country: Some(CountryCode::US),
            line1: None,
            line2: None,
            postal_code: None,
            state: None,
        })
    );
    assert_eq!(
        customer2.tax_id,
        Some(TaxId {
            type_: orb_billing::TaxIdType::UsEin,
            value: "12-3456789".into(),
            country: CountryCode::US,
        })
    );

    // List customers, and ensure we see both customers that we created.
    // Do so with a page size of one to exercise the pagination logic.
    let customer_ids: HashSet<_> = client
        .list_customers(&ListParams::default().page_size(1))
        .map_ok(|customer| customer.id)
        .try_collect()
        .await
        .unwrap();
    assert!(customer_ids.contains(&customer.id));
    assert!(customer_ids.contains(&customer2.id));
}

#[test(tokio::test)]
async fn test_plans() {
    let client = new_client();

    let plans: Vec<_> = client.list_plans(&MAX_PAGE_LIST_PARAMS).try_collect().await.unwrap();
    println!("plans = {:#?}", plans);

    // TODO: validate list results.
    // TODO: test get_plan.
    // TODO: test get_plan_by_external_id.
    // Testing the above will be hard as there is no API to create plans.
}

#[test(tokio::test)]
async fn test_subscriptions() {
    let client = new_client();

    let subscriptions: Vec<_> = client.list_subscriptions(&SubscriptionListParams::default()).try_collect().await.unwrap();
    println!("subscriptions = {:#?}", subscriptions);

    // TODO: validate list results.
    // TODO: test get_subscription.
}

#[test(tokio::test)]
async fn test_invoices() {
    let client = new_client();

    let invoices: Vec<_> = client.list_invoices(&InvoiceListParams::default()).try_collect().await.unwrap();
    println!("invoices = {:#?}", invoices);

    // TODO: validate list results.
    // TODO: test get_invoice.
}


#[test(tokio::test)]
async fn test_errors() {
    let client = new_client();

    let res = client.get_customer("$NOEXIST$").await;
    assert_error_with_status_code(res, StatusCode::NOT_FOUND);

    let res = client.get_customer_by_external_id("$NOEXIST$").await;
    assert_error_with_status_code(res, StatusCode::NOT_FOUND);
}
