use futures_core::Stream;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_enum_str::{Deserialize_enum_str, Serialize_enum_str};
use time::OffsetDateTime;
use crate::{Client, Error, ListParams};
use crate::util::StrIteratorExt;

const BACKFILLS: [&str; 2] = ["events", "backfills"];
const CLOSE: &str = "close";
const REVERT: &str = "revert";

/// Create a backfill - see https://docs.withorb.com/reference/create-backfill
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CreateBackfillParams {
    /// If true, replaces all existing events in the timeframe with the newly ingested events. If false, adds the newly ingested events to the existing events.
    pub replace_existing_events: bool,
    /// The (inclusive) start of the usage timeframe affected by this backfill.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_start: OffsetDateTime,
    /// The (exclusive) end of the usage timeframe affected by this backfill.
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_end: OffsetDateTime,
    /// The time at which no more events will be accepted for this backfill. The backfill will automatically begin reflecting throughout Orb at the close time. If not specified, it will default to 1 day after the creation of the backfill.
    #[serde(with = "time::serde::rfc3339::option")]
    pub close_time: Option<OffsetDateTime>,
    /// The ID of the customer to which this backfill is scoped. (cannot be used with external_customer_id)
    pub customer_id: Option<String>,
    /// The external customer ID of the customer to which this backfill is scoped. (cannot be used with customer_id)
    pub external_customer_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Deserialize_enum_str, Serialize_enum_str)]
#[serde(rename_all = "snake_case")]
pub enum BackfillStatus {
    Pending,
    Reflected,
    PendingRevert,
    Reverted,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BackfillStatusResponse {
    /// The backfill id
    pub id: String,
    /// The status of the backfill.
    pub status: BackfillStatus,
    /// If in the future, the time at which the backfill will automatically close. If in the past, the time at which the backfill was closed.
    #[serde(with = "time::serde::rfc3339")]
    pub close_time: OffsetDateTime,
    /// The time at which this backfill was reverted.
    #[serde(with = "time::serde::rfc3339::option")]
    pub reverted_at: Option<OffsetDateTime>,
    /// Undocumented
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_start: OffsetDateTime,
    /// Undocumented
    #[serde(with = "time::serde::rfc3339")]
    pub timeframe_end: OffsetDateTime,
    /// Undocumented
    #[serde(with = "time::serde::rfc3339")]
    pub created_at: OffsetDateTime,
    /// The customer ID this backfill is scoped to. If null, this backfill is not scoped to a single customer.
    pub customer_id: Option<String>
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListBackfillsResponse {
    pub data: Vec<BackfillStatusResponse>
}

impl Client {
    /// See https://docs.withorb.com/reference/create-backfill
    pub async fn create_backfill(&self, params: &CreateBackfillParams) -> Result<BackfillStatusResponse, Error> {
        let req = self.build_request(Method::POST, BACKFILLS);
        let req = req.json(params);
        self.send_request(req).await
    }

    /// See https://docs.withorb.com/reference/close-backfill
    pub async fn close_backfill(&self, backfill_id: String) -> Result<BackfillStatusResponse, Error> {
        let req = self.build_request(Method::POST, BACKFILLS.chain_one(backfill_id).chain_one(CLOSE));
        self.send_request(req).await
    }

    /// See https://docs.withorb.com/reference/revert-backfill
    pub async fn revert_backfill(&self, backfill_id: String) -> Result<BackfillStatusResponse, Error> {
        let req = self.build_request(Method::POST, BACKFILLS.chain_one(backfill_id).chain_one(REVERT));
        self.send_request(req).await
    }

    /// See https://docs.withorb.com/reference/list-backfills
    pub fn list_backfills(&self) -> impl Stream<Item = Result<BackfillStatusResponse, Error>> + '_ {
        let req = self.build_request(Method::GET, BACKFILLS);
        self.stream_paginated_request(&ListParams::default(), req)
    }

    /// See https://docs.withorb.com/reference/fetch-backfill
    pub async fn fetch_backfill(&self, backfill_id: String) -> Result<BackfillStatusResponse, Error> {
        let req = self.build_request(Method::GET, BACKFILLS.chain_one(backfill_id));
        self.send_request(req).await
    }
}