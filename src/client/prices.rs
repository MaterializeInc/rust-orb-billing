use serde::{Deserialize, Serialize};

/// An Orb price
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum Price {
    /// Used to represent unit prices
    #[serde(rename = "unit")]
    Unit(UnitPrice),
    /// Used to represent tiered prices
    #[serde(rename = "tiered")]
    Tiered(TieredPrice),
    // TODO: Add support for additional prices
}

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
pub enum PriceOverride {
    /// Used to override unit prices
    #[serde(rename = "unit")]
    Unit(OverrideUnitPrice),
    // TODO: Add support for additional price overrides
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct OverrideUnitPrice {
    /// Id of the price
    pub id: String,
    /// Will be "unit" for this type of price override
    pub model_type: String,
    /// The starting quantity of the price
    pub fixed_price_quantity: Option<u64>,
    pub unit_config: UnitConfig,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct UnitConfig {
    /// Rate per unit of usage
    pub unit_amount: String,
    /// Multiplier to scale rated quantity by
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scaling_factor: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct TieredConfig {
    /// Tiers for rating based on total usage quantities into the specified tier
    pub tiers: Vec<Tier>
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub struct Tier {
    /// Inclusive tier starting value
    pub first_unit: u64,
    /// Exclusive tier ending value. If null, this is treated as the last tier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_unit: Option<u64>,
    /// Rate per unit of usage
    pub unit_amount: String,
}