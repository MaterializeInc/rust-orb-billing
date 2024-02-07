use serde::{Deserialize, Serialize};

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
    unit_config: UnitConfig,
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
    tiered_config: TieredConfig,
    // TODO: many missing fields.
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