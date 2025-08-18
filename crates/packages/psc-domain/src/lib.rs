use anyhow::Result;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Money {
    amount: Decimal,
    currency: &'static str,
}

impl Money {
    pub fn new(amount: Decimal, currency: &'static str) -> Self {
        Self { amount, currency }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> &'static str {
        self.currency
    }

    pub fn add(&self, other: &Self) -> Result<Self> {
        if self.currency != other.currency {
            return Err(anyhow::anyhow!(
                "Cannot add money with different currencies"
            ));
        }
        Ok(Self {
            amount: self.amount + other.amount,
            currency: self.currency,
        })
    }

    pub fn sub(&self, other: &Self) -> Result<Self> {
        if self.currency != other.currency {
            return Err(anyhow::anyhow!(
                "Cannot subtract money with different currencies"
            ));
        }
        Ok(Self {
            amount: self.amount - other.amount,
            currency: self.currency,
        })
    }
}

use uuid::Uuid;

macro_rules! impl_id {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $name(Uuid);

        impl $name {
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}", self.0)
            }
        }
    };
}

impl_id!(PrincipalID);
impl_id!(ProviderWalletID);
impl_id!(LedgerAccountID);
