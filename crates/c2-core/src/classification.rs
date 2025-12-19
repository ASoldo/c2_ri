use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityClassification {
    Unclassified,
    Controlled,
    Restricted,
    Confidential,
    Secret,
    TopSecret,
}

impl Default for SecurityClassification {
    fn default() -> Self {
        Self::Unclassified
    }
}

impl FromStr for SecurityClassification {
    type Err = ();

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.to_ascii_lowercase().as_str() {
            "unclassified" => Ok(Self::Unclassified),
            "controlled" => Ok(Self::Controlled),
            "restricted" => Ok(Self::Restricted),
            "confidential" => Ok(Self::Confidential),
            "secret" => Ok(Self::Secret),
            "top_secret" | "top-secret" | "topsecret" => Ok(Self::TopSecret),
            _ => Err(()),
        }
    }
}
