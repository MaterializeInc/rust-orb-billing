use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

use crate::client::Client;
use crate::config::ListParams;
use crate::error::Error;

const COUPONS_PATH: [&str; 1] = ["coupons"];

/// An Orb coupon
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Coupon {
    /// Id of the coupon
    pub id: String,
    /// User-facing coupon code
    pub redemption_code: String,
    /// This allows for a coupon's discount to apply for a limited time
    /// (determined in months); a null value here means "unlimited time".
    pub duration_in_months: Option<serde_json::Number>,
    /// The type of discount
    pub discount: Discount,
}

/// An Orb discount type
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "discount_type")]
pub enum Discount {
    /// Percentage-based discount
    #[serde(rename = "percentage")]
    Percentage(PercentageDiscount),
    // TODO: Add support for amount-based discount
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PercentageDiscount {
    pub applies_to_price_ids: Vec<String>,
    pub percentage_discount: serde_json::Number,
}

/// Coupon redeemed on a subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct RedeemedCoupon {
    /// Id of the coupon
    pub coupon_id: String,
    #[serde(with = "time::serde::rfc3339")]
    /// The date when the coupon started applying
    pub start_date: OffsetDateTime,
    /// The date when the coupon stops applying
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
}

/// Parameters for a coupon list operation.
#[derive(Debug, Clone)]
pub struct CouponListParams<'a> {
    inner: ListParams,
    redemption_code_filter: Option<&'a str>,
    show_archived_filter: Option<bool>,
}

impl<'a> Default for CouponListParams<'a> {
    fn default() -> CouponListParams<'a> {
        CouponListParams::DEFAULT
    }
}

impl<'a> CouponListParams<'a> {
    /// The default coupon list parameters.
    ///
    /// Exposed as a constant for use in constant evaluation contexts.
    pub const DEFAULT: CouponListParams<'static> = CouponListParams {
        inner: ListParams::DEFAULT,
        redemption_code_filter: None,
        show_archived_filter: None,
    };

    /// Sets the page size for the list operation.
    ///
    /// See [`ListParams::page_size`].
    pub const fn page_size(mut self, page_size: u64) -> Self {
        self.inner = self.inner.page_size(page_size);
        self
    }


    /// Filters the listing to the specified redemption code.
    pub const fn redemption_code(mut self, filter: &'a str) -> Self {
        self.redemption_code_filter = Some(filter);
        self
    }

    /// Filters the listing to the specified show_archived.
    pub const fn show_archived(mut self, filter: bool) -> Self {
        self.show_archived_filter = Some(filter);
        self
    }
}

impl Client {
    /// Lists all coupons.
    ///
    /// The underlying API call is paginated. The returned stream will fetch
    /// additional pages as it is consumed.
    pub fn list_coupons(&self, params: &CouponListParams) -> impl Stream<Item = Result<Coupon, Error>> + '_ {
        let req = self.build_request(Method::GET, COUPONS_PATH);
        let req = match params.redemption_code_filter {
            Some(redemption_code) => req.query(&[("redemption_code", redemption_code)]),
            None => req,
        };
        let req = match params.show_archived_filter {
            Some(show_archived) => req.query(&[("show_archived", show_archived)]),
            None => req,
        };
        self.stream_paginated_request(&params.inner, req)
    }
}