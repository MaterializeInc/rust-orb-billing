use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

/// An Orb price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "model_type")]
pub enum Price {
    /// Used to represent unit prices
    #[serde(rename = "unit")]
    Unit(UnitPrice),
    /// Used to represent tiered prices
    #[serde(rename = "tiered")]
    Tiered(TieredPrice),
    // TODO: Add support for additional prices
}

/// With unit pricing, each unit costs a fixed amount.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UnitPrice {
    /// Id of the price
    pub id: String,
    /// Name of the price
    pub name: String,
    /// Config with rates per unit
    pub unit_config: UnitConfig,
    /// Which phase of the plan this price is associated with
    pub plan_phase_order: Option<i64>,
    // TODO: many missing fields.
}

/// In tiered pricing, the cost of a given unit depends on the tier range that it
/// falls into, where each tier range is defined by an upper and lower bound.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TieredPrice {
    /// Id of the price
    pub id: String,
    /// Name of the price
    pub name: String,
    /// Config with rates per tier
    pub tiered_config: TieredConfig,
    /// Which phase of the plan this price is associated with
    pub plan_phase_order: Option<i64>,
    // TODO: many missing fields.
}


/// An Orb price interval
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct PriceInterval {
    /// The id of the price interval.
    pub id: String,
    /// The price of the interval.
    pub price: Price,
    /// The start date of the price interval.
    /// This is the date that Orb starts billing for this price.
    #[serde(with = "time::serde::rfc3339")]
    pub start_date: OffsetDateTime,
    /// The end date of the price interval.
    /// This is the date that Orb stops billing for this price.
    #[serde(with = "time::serde::rfc3339::option")]
    pub end_date: Option<OffsetDateTime>,
}

/// A list of price intervals to edit on the subscription.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct EditPriceInterval {
    /// The id of the price interval to edit.
    pub price_interval_id: String,
    /// A list of fixed fee quantity transitions to use for this price interval.
    /// Note that this list will overwrite all existing fixed fee quantity transitions on the price interval.
    pub fixed_fee_quantity_transitions: Option<Vec<FixedFeeQuantityTransition>>,
}

/// A fixed fee quantity transition is used to update the quantity for a price interval.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct FixedFeeQuantityTransition {
    /// The quantity of the fixed fee quantity transition.
    pub quantity: serde_json::Number,
    /// The date that the fixed fee quantity transition should take effect.
    pub effective_date: String,
}


/// Price overrides are used to update some or all prices in a plan for the specific subscription being created.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
#[serde(tag = "model_type")]
pub enum PriceOverride {
    /// Used to override unit prices
    #[serde(rename = "unit")]
    Unit(OverrideUnitPrice),
    // TODO: Add support for additional price overrides
}

/// Price override for a unit price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct OverrideUnitPrice {
    /// Id of the price
    pub id: String,
    /// Will be "unit" for this type of price override
    pub model_type: String,
    /// The starting quantity of the price
    pub fixed_price_quantity: Option<serde_json::Number>,
    /// Configuration for a unit price
    pub unit_config: UnitConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UnitConfig {
    /// Rate per unit of usage
    pub unit_amount: String,
    /// Multiplier to scale rated quantity by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_factor: Option<serde_json::Number>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TieredConfig {
    /// Tiers for rating based on total usage quantities into the specified tier
    pub tiers: Vec<Tier>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Tier {
    /// Inclusive tier starting value
    pub first_unit: serde_json::Number,
    /// Exclusive tier ending value. If null, this is treated as the last tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_unit: Option<serde_json::Number>,
    /// Rate per unit of usage
    pub unit_amount: String,
}