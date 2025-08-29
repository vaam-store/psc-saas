use rust_decimal::Decimal;
use rust_decimal::prelude::FromPrimitive;
use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, PartialOrd, Ord)]
pub struct Money {
    amount: Decimal,
    currency: &'static str,
}

impl Money {
    pub fn new(amount: i64, currency: &'static str) -> Self {
        Self {
            amount: Decimal::from(amount),
            currency,
        }
    }

    pub fn zero(currency: &'static str) -> Self {
        Self {
            amount: Decimal::ZERO,
            currency,
        }
    }

    pub fn amount(&self) -> Decimal {
        self.amount
    }

    pub fn currency(&self) -> &'static str {
        self.currency
    }

    pub fn multiply_percent(&self, percent: f64) -> Self {
        let percentage = Decimal::from_f64(percent / 100.0).unwrap();
        Self {
            amount: self.amount * percentage,
            currency: self.currency,
        }
    }
}

impl Add for Money {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        if self.currency != other.currency {
            panic!("Cannot add money with different currencies");
        }
        Self {
            amount: self.amount + other.amount,
            currency: self.currency,
        }
    }
}

impl AddAssign for Money {
    fn add_assign(&mut self, other: Self) {
        if self.currency != other.currency {
            panic!("Cannot add money with different currencies");
        }
        self.amount += other.amount;
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
