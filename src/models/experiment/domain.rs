use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Variant {
    pub key: String,
    pub is_control: bool,
    #[serde(default = "default_config")]
    pub config: serde_json::Value,
}

fn default_config() -> serde_json::Value {
    serde_json::json!({})
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Segment {
    pub priority: i32,
    pub rollout_percent: u32,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    pub distributions: Vec<Distribution>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ConstraintOperator {
    Eq,
    Neq,
    Gt,
    Gte,
    Lt,
    Lte,
    In,
    NotIn,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
pub struct Constraint {
    pub property: String,
    pub operator: ConstraintOperator,
    pub value: serde_json::Value,
}

#[derive(Serialize, Deserialize, Clone, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Distribution {
    pub variant_key: String,
    pub percent: u32,
}
