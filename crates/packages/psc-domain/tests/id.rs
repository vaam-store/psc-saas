use psc_domain::{LedgerAccountID, PrincipalID, ProviderWalletID};

#[test]
fn test_principal_id() {
    let id1 = PrincipalID::new();
    let id2 = PrincipalID::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_provider_wallet_id() {
    let id1 = ProviderWalletID::new();
    let id2 = ProviderWalletID::new();
    assert_ne!(id1, id2);
}

#[test]
fn test_ledger_account_id() {
    let id1 = LedgerAccountID::new();
    let id2 = LedgerAccountID::new();
    assert_ne!(id1, id2);
}
