use psc_domain::Money;

#[test]
fn test_add_money() {
    let m1 = Money::new(10050, "XAF");
    let m2 = Money::new(5025, "XAF");
    let result = m1 + m2;
    assert_eq!(result.amount().to_string(), "150.75");
    assert_eq!(result.currency(), "XAF");
}

#[test]
#[should_panic(expected = "Cannot add money with different currencies")]
fn test_add_money_different_currencies() {
    let m1 = Money::new(10050, "XAF");
    let m2 = Money::new(5025, "USD");
    let _ = m1 + m2;
}

#[test]
fn test_money_comparison() {
    let m1 = Money::new(10000, "XAF");
    let m2 = Money::new(5000, "XAF");
    let m3 = Money::new(10000, "XAF");

    assert!(m1 > m2);
    assert!(m2 < m1);
    assert!(m1 >= m3);
    assert!(m1 <= m3);
    assert_eq!(m1, m3);
}

#[test]
fn test_money_zero() {
    let zero_xaf = Money::zero("XAF");
    assert_eq!(zero_xaf.amount().to_string(), "0");
    assert_eq!(zero_xaf.currency(), "XAF");
}

#[test]
fn test_money_multiply_percent() {
    let amount = Money::new(10000, "XAF"); // 100.00 XAF
    let fee = amount.multiply_percent(1.5); // 1.5% of 100.00 = 1.50 XAF
    assert_eq!(fee.amount().to_string(), "1.5");
    assert_eq!(fee.currency(), "XAF");

    let amount_large = Money::new(100000000, "XAF"); // 1,000,000.00 XAF
    let fee_large = amount_large.multiply_percent(0.25); // 0.25% of 1,000,000.00 = 2,500.00 XAF
    assert_eq!(fee_large.amount().to_string(), "2500");
    assert_eq!(fee_large.currency(), "XAF");

    let amount_small = Money::new(10, "XAF"); // 0.10 XAF
    let fee_small = amount_small.multiply_percent(5.0); // 5% of 0.10 = 0.005 XAF
    assert_eq!(fee_small.amount().to_string(), "0.005");
    assert_eq!(fee_small.currency(), "XAF");
}
