#![deny(clippy::all)]
#![forbid(unsafe_code)]

//! A shared library for calculating various types of fees based on configurable rules.

use psc_domain::Money;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum FeeError {
    #[error("Invalid percentage value: {0}. Must be between 0.0 and 100.0")]
    InvalidPercentage(f64),
    #[error("Tiered fees must be sorted by threshold")]
    UnsortedTiers,
}

/// Represents a rule for calculating a fee.
#[derive(Debug, Clone, PartialEq)]
pub enum FeeRule {
    /// A fixed fee amount.
    Fixed(Money),
    /// A fee calculated as a percentage of the transaction amount.
    /// The value should be between 0.0 and 100.0.
    Percentage {
        value: f64,
        min: Option<Money>,
        max: Option<Money>,
    },
    /// A fee that varies based on the transaction amount.
    /// The tiers must be sorted by their `up_to` threshold.
    Tiered { tiers: Vec<Tier> },
}

/// Represents a single tier in a tiered fee structure.
#[derive(Debug, Clone, PartialEq)]
pub struct Tier {
    /// The upper bound for this tier (inclusive).
    pub up_to: Money,
    /// The fee to apply for amounts within this tier.
    pub fee: Money,
}

impl FeeRule {
    /// Calculates the fee for a given amount based on the rule.
    pub fn calculate(&self, amount: Money) -> Result<Money, FeeError> {
        match self {
            FeeRule::Fixed(fee) => Ok(*fee),
            FeeRule::Percentage { value, min, max } => {
                if !(0.0..=100.0).contains(value) {
                    return Err(FeeError::InvalidPercentage(*value));
                }
                let mut fee = amount.multiply_percent(*value);
                if let Some(min_fee) = min {
                    if fee < *min_fee {
                        fee = *min_fee;
                    }
                }
                if let Some(max_fee) = max {
                    if fee > *max_fee {
                        fee = *max_fee;
                    }
                }
                Ok(fee)
            }
            FeeRule::Tiered { tiers } => {
                // Ensure tiers are sorted
                for i in 1..tiers.len() {
                    if tiers[i - 1].up_to > tiers[i].up_to {
                        return Err(FeeError::UnsortedTiers);
                    }
                }

                for tier in tiers {
                    if amount <= tier.up_to {
                        return Ok(tier.fee);
                    }
                }
                // If amount is greater than all tiers, return the fee for the highest tier
                tiers
                    .last()
                    .map(|t| t.fee)
                    .ok_or_else(|| FeeError::UnsortedTiers) // Should not happen if tiers is not empty
            }
        }
    }
}

/// Calculates the total fee for a given amount by applying a set of fee rules.
///
/// # Arguments
///
/// * `amount` - The transaction amount.
/// * `rules` - A slice of `FeeRule`s to apply.
///
/// # Returns
///
/// The total calculated fee, or an error if any of the rules are invalid.
pub fn calculate_fee(amount: Money, rules: &[FeeRule]) -> Result<Money, FeeError> {
    let mut total_fee = Money::zero("XAF");
    for rule in rules {
        total_fee = total_fee + rule.calculate(amount)?;
    }
    Ok(total_fee)
}

#[cfg(test)]
mod tests {
    use super::*;
    use psc_domain::Money;

    #[test]
    fn test_fixed_fee() {
        let amount = Money::new(10000, "XAF");
        let rule = FeeRule::Fixed(Money::new(100, "XAF"));
        let fee = calculate_fee(amount, &[rule]).unwrap();
        assert_eq!(fee, Money::new(100, "XAF"));
    }

    #[test]
    fn test_percentage_fee() {
        let amount = Money::new(10000, "XAF");
        let rule = FeeRule::Percentage {
            value: 1.5,
            min: None,
            max: None,
        };
        let fee = calculate_fee(amount, &[rule]).unwrap();
        assert_eq!(fee, Money::new(150, "XAF"));
    }

    #[test]
    fn test_percentage_fee_with_min_cap() {
        let amount = Money::new(1000, "XAF");
        let rule = FeeRule::Percentage {
            value: 1.0,
            min: Some(Money::new(50, "XAF")),
            max: None,
        };
        let fee = calculate_fee(amount, &[rule]).unwrap();
        assert_eq!(fee, Money::new(50, "XAF"));
    }

    #[test]
    fn test_percentage_fee_with_max_cap() {
        let amount = Money::new(100000, "XAF");
        let rule = FeeRule::Percentage {
            value: 2.0,
            min: None,
            max: Some(Money::new(1500, "XAF")),
        };
        let fee = calculate_fee(amount, &[rule]).unwrap();
        assert_eq!(fee, Money::new(1500, "XAF"));
    }

    #[test]
    fn test_invalid_percentage() {
        let amount = Money::new(10000, "XAF");
        let rule = FeeRule::Percentage {
            value: 101.0,
            min: None,
            max: None,
        };
        let result = calculate_fee(amount, &[rule]);
        assert_eq!(result, Err(FeeError::InvalidPercentage(101.0)));
    }

    #[test]
    fn test_tiered_fee() {
        let tiers = vec![
            Tier {
                up_to: Money::new(5000, "XAF"),
                fee: Money::new(50, "XAF"),
            },
            Tier {
                up_to: Money::new(20000, "XAF"),
                fee: Money::new(100, "XAF"),
            },
            Tier {
                up_to: Money::new(50000, "XAF"),
                fee: Money::new(200, "XAF"),
            },
        ];
        let rule = FeeRule::Tiered { tiers };

        let amount1 = Money::new(4000, "XAF");
        let fee1 = calculate_fee(amount1, &[rule.clone()]).unwrap();
        assert_eq!(fee1, Money::new(50, "XAF"));

        let amount2 = Money::new(20000, "XAF");
        let fee2 = calculate_fee(amount2, &[rule.clone()]).unwrap();
        assert_eq!(fee2, Money::new(100, "XAF"));

        let amount3 = Money::new(60000, "XAF");
        let fee3 = calculate_fee(amount3, &[rule.clone()]).unwrap();
        assert_eq!(fee3, Money::new(200, "XAF"));
    }

    #[test]
    fn test_unsorted_tiers() {
        let tiers = vec![
            Tier {
                up_to: Money::new(20000, "XAF"),
                fee: Money::new(100, "XAF"),
            },
            Tier {
                up_to: Money::new(5000, "XAF"),
                fee: Money::new(50, "XAF"),
            },
        ];
        let rule = FeeRule::Tiered { tiers };
        let amount = Money::new(4000, "XAF");
        let result = calculate_fee(amount, &[rule]);
        assert_eq!(result, Err(FeeError::UnsortedTiers));
    }

    #[test]
    fn test_combined_fees() {
        let amount = Money::new(10000, "XAF");
        let rules = vec![
            FeeRule::Fixed(Money::new(25, "XAF")),
            FeeRule::Percentage {
                value: 1.0,
                min: None,
                max: None,
            },
        ];
        let fee = calculate_fee(amount, &rules).unwrap();
        assert_eq!(fee, Money::new(125, "XAF"));
    }

    #[test]
    fn test_zero_amount() {
        let amount = Money::zero("XAF");
        let rules = vec![
            FeeRule::Fixed(Money::new(50, "XAF")),
            FeeRule::Percentage {
                value: 2.0,
                min: None,
                max: None,
            },
        ];
        let fee = calculate_fee(amount, &rules).unwrap();
        assert_eq!(fee, Money::new(50, "XAF"));
    }
}
