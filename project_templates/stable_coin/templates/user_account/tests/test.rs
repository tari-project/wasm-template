mod support;

use support::UserAccountTest;
use tari_template_lib::args;
use tari_template_lib::models::{Amount, VaultId};
use tari_template_lib::prelude::RistrettoPublicKeyBytes;
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_test_tooling::support::confidential::generate_withdraw_proof;
use tari_transaction::Transaction;
use tari_dan_common_types::DerivableFromPublicKey;

#[test]
fn it_creates_and_funds_a_user_account() {
    let mut test = UserAccountTest::new();

    let result = test.open_accounts(vec![test.test.get_test_public_key_bytes()]);
    let user_account_component = result[0];
    test.fund_account(user_account_component, Amount(500));
}

#[test]
fn it_allows_a_user_to_transact() {
    let mut test = UserAccountTest::new();
    let (alice_proof, alice_pk, alice_key) = test.test.create_owner_proof();
    let alice_pk = RistrettoPublicKeyBytes::derive_from_public_key(&alice_pk);
    let (_bob_proof, bob_pk, bob_key) = test.test.create_owner_proof();
    let bob_pk = RistrettoPublicKeyBytes::derive_from_public_key(&bob_pk);

    let accounts = test.open_accounts(vec![alice_pk, bob_pk]);
    let (alice_account, bob_account) = (accounts[0], accounts[1]);

    let output_mask = test.fund_account(alice_account, Amount(500));

    let alice_to_bob_proof = generate_withdraw_proof(
        &output_mask,
        Amount(50),
        Some(Amount(500 - 50)),
        Amount::zero(),
    );
    // Transfer to bob
    test.test.execute_expect_success(
        Transaction::builder()
            .call_method(
                alice_account,
                "transfer_to",
                args![bob_account, alice_to_bob_proof.proof],
            )
            .build_and_seal(&alice_key),
        vec![alice_proof],
    );
}

#[test]
fn it_rejects_transaction_if_dest_is_on_deny_list() {
    let mut test = UserAccountTest::new();
    let (alice_proof, alice_pk, alice_key) = test.test.create_owner_proof();
    let alice_pk = RistrettoPublicKeyBytes::derive_from_public_key(&alice_pk);
    let (_bob_proof, bob_pk, bob_key) = test.test.create_owner_proof();
    let bob_pk = RistrettoPublicKeyBytes::derive_from_public_key(&bob_pk);

    let accounts = test.open_accounts(vec![alice_pk, bob_pk]);
    let (alice_account, bob_account) = (accounts[0], accounts[1]);

    let bob_vault: VaultId = test
        .test
        .extract_component_value(bob_account, "$.user_badge");

    let output_mask = test.fund_account(alice_account, Amount(500));
    test.add_account_to_deny_list(bob_pk, bob_account, bob_vault);

    let alice_to_bob_proof = generate_withdraw_proof(
        &output_mask,
        Amount(50),
        Some(Amount(500 - 50)),
        Amount::zero(),
    );
    // Transfer to Bob fails
    let reason = test.test.execute_expect_failure(
        Transaction::builder()
            .call_method(
                alice_account,
                "transfer_to",
                args![bob_account, alice_to_bob_proof.proof],
            )
            .build_and_seal(&alice_key),
        vec![alice_proof.clone()],
    );

    assert_reject_reason(reason, format!("Transfer denied to account {bob_account}"));

    test.remove_account_from_deny_list(bob_pk);
    // Transfer succeeds
    test.test.execute_expect_success(
        Transaction::builder()
            .call_method(
                alice_account,
                "transfer_to",
                args![bob_account, alice_to_bob_proof.proof],
            )
            .build_and_seal(&alice_key),
        vec![alice_proof],
    );
}
