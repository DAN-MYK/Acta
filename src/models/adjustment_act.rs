use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::models::DocumentDirection;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "adjustment_act_status", rename_all = "lowercase")]
pub enum AdjustmentActStatus {
    Draft,
    Issued,
    Signed,
    Applied,
}

impl AdjustmentActStatus {
    pub fn can_transition_to(&self, next: &AdjustmentActStatus) -> bool {
        matches!(
            (self, next),
            (Self::Draft, Self::Issued)
                | (Self::Issued, Self::Signed)
                | (Self::Signed, Self::Applied)
        )
    }

    pub fn next(&self) -> Option<AdjustmentActStatus> {
        match self {
            Self::Draft => Some(Self::Issued),
            Self::Issued => Some(Self::Signed),
            Self::Signed => Some(Self::Applied),
            Self::Applied => None,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Draft => "Чернетка",
            Self::Issued => "Виставлено",
            Self::Signed => "Підписано",
            Self::Applied => "Застосовано",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Issued => "issued",
            Self::Signed => "signed",
            Self::Applied => "applied",
        }
    }
}

impl std::fmt::Display for AdjustmentActStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdjustmentAct {
    pub id: Uuid,
    pub company_id: Uuid,
    pub original_act_id: Uuid,
    pub counterparty_id: Uuid,
    pub number: String,
    pub date: NaiveDate,
    pub direction: DocumentDirection,
    pub total_amount: Decimal,
    pub status: AdjustmentActStatus,
    pub notes: Option<String>,
    pub bas_id: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct AdjustmentActItem {
    pub id: Uuid,
    pub adjustment_act_id: Uuid,
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
    pub total_price: Decimal,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AdjustmentActListRow {
    pub id: Uuid,
    pub company_id: Uuid,
    pub original_act_id: Uuid,
    pub original_act_number: String,
    pub counterparty_id: Uuid,
    pub counterparty_name: String,
    pub number: String,
    pub date: NaiveDate,
    pub total_amount: Decimal,
    pub direction: DocumentDirection,
    pub status: AdjustmentActStatus,
}

pub struct NewAdjustmentActItem {
    pub description: String,
    pub quantity: Decimal,
    pub unit_price: Decimal,
}

pub struct UpdateAdjustmentAct {
    pub number: String,
    pub date: NaiveDate,
    pub notes: Option<String>,
}
