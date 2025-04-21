mod setup;

use crate::setup::IssuerTest;
use tari_template_lib::args;
use tari_template_lib::models::Amount;
use tari_template_test_tooling::support::confidential::{
    generate_confidential_proof, generate_withdraw_proof,
};
use tari_transaction::Transaction;

#[test]
fn it_increases_and_decreases_supply() {
    let IssuerTest {
        mut test,
        stable_coin_issuer_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        ..
    } = IssuerTest::new();

    let (output, mask, _) = generate_confidential_proof(Amount(1_000_000), None);

    test.execute_expect_success(
        Transaction::builder()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_issuer_component,
                "increase_supply",
                args![output],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof.clone()],
    );

    // TODO: cannot get supply on confidential asset yet

    let proof = generate_withdraw_proof(&mask, Amount(1_000_000), None, Amount::zero());

    test.execute_expect_success(
        Transaction::builder()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(
                stable_coin_issuer_component,
                "decrease_supply",
                args![proof.proof],
            )
            .drop_all_proofs_in_workspace()
            .build_and_seal(&admin_key),
        vec![admin_proof],
    );
}
