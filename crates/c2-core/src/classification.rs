use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
