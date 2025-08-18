use psc_domain::Money;
use rust_decimal_macros::dec;

#[test]
fn test_add_money() {
    let m1 = Money::new(dec!(100.50), "XAF");
    let m2 = Money::new(dec!(50.25), "XAF");
    let result = m1.add(&m2).unwrap();
    assert_eq!(result.amount(), dec!(150.75));
    assert_eq!(result.currency(), "XAF");
}

#[test]
fn test_sub_money() {
    let m1 = Money::new(dec!(100.50), "XAF");
    let m2 = Money::new(dec!(50.25), "XAF");
    let result = m1.sub(&m2).unwrap();
    assert_eq!(result.amount(), dec!(50.25));
    assert_eq!(result.currency(), "XAF");
}

#[test]
fn test_add_money_different_currencies() {
    let m1 = Money::new(dec!(100.50), "XAF");
    let m2 = Money::new(dec!(50.25), "USD");
    let result = m1.add(&m2);
    assert!(result.is_err());
}

#[test]
fn test_sub_money_different_currencies() {
    let m1 = Money::new(dec!(100.50), "XAF");
    let m2 = Money::new(dec!(50.25), "USD");
    let result = m1.sub(&m2);
    assert!(result.is_err());
}
