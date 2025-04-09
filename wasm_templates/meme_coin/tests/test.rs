use std::ops::Sub;
use tari_engine_types::commit_result::RejectReason;
use tari_template_lib::args;
use tari_template_lib::models::NonFungibleAddress;
use tari_template_lib::prelude::{Amount, Bucket, ComponentAddress};
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::TemplateTest;
use tari_transaction::Transaction;

struct CreateMemeCoinResult {
    pub initial_supply: Amount,
    pub admin_account_component: ComponentAddress,
    pub admin_account_proof: NonFungibleAddress,
    pub admin_account_secret: RistrettoSecretKey,
    pub meme_coin_component: ComponentAddress,
}

fn create_meme_coin(test: &mut TemplateTest, name: &str) -> CreateMemeCoinResult {
    let initial_supply = Amount(1_000_000_000_000);
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    // create new memecoin
    let create_coin_result = test.execute_expect_success(
        Transaction::builder()
            .call_function(
                test.get_template_address("{{ project-name | upper_camel_case }}"),
                "mint",
                args![initial_supply, name.to_string(), None::<String>],
            )
            .put_last_instruction_output_on_workspace("ret")
            .call_method(account_component, "deposit", args![Workspace("ret.1")])
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    let (coin_component, _) = create_coin_result.finalize.execution_results[0]
        .decode::<(ComponentAddress, Bucket)>()
        .unwrap();

    CreateMemeCoinResult {
        initial_supply,
        admin_account_component: account_component,
        admin_account_proof: owner_proof,
        admin_account_secret: account_secret_key,
        meme_coin_component: coin_component,
    }
}

#[test]
fn test_memecoin_owner_only_allowed_method() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | upper_case }}");

    // make sure that admin only method is working
    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "burn",
                args![Amount::new(10)],
            )
            .build_and_seal(&meme_coin_result.admin_account_secret),
        vec![meme_coin_result.admin_account_proof],
    );
    assert!(result.finalize.result.is_accept());

    // test if a new user can call it
    let (_, owner_proof, account_secret_key) = template_test.create_funded_account();
    let reject_reason = template_test.execute_expect_failure(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "burn",
                args![Amount::new(10)],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(matches!(reject_reason, RejectReason::ExecutionFailure(_)));
    if let RejectReason::ExecutionFailure(reason) = reject_reason {
        assert!(reason.starts_with("Access Denied:"));
        assert!(reason.contains(
            format!(
                "call component method 'burn' on {}",
                meme_coin_result.meme_coin_component
            )
                .as_str()
        ));
    }
}

#[test]
fn test_memecoin_owner_transfer_coins() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | upper_case }}");
    let (target_account_addr, _, _) = template_test.create_empty_account();

    let withdraw_amount = Amount::new(10);

    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "withdraw",
                args![withdraw_amount.clone()],
            )
            .put_last_instruction_output_on_workspace("withdrawn_bucket")
            .call_method(
                target_account_addr,
                "deposit",
                args![Workspace("withdrawn_bucket")],
            )
            .call_method(
                meme_coin_result.meme_coin_component,
                "vault_address",
                args![],
            )
            .put_last_instruction_output_on_workspace("coin_vault_address")
            .call_method(
                target_account_addr,
                "balance",
                args![Workspace("coin_vault_address")],
            )
            .call_method(meme_coin_result.meme_coin_component, "balance", args![])
            .build_and_seal(&meme_coin_result.admin_account_secret),
        vec![meme_coin_result.admin_account_proof],
    );
    assert!(result.finalize.result.is_accept());

    let target_account_memecoin_balance = result.finalize.execution_results[5]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(target_account_memecoin_balance, withdraw_amount);

    let memecoin_balance = result.finalize.execution_results[6]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(
        memecoin_balance,
        meme_coin_result.initial_supply.sub(withdraw_amount)
    );
}

#[test]
fn test_memecoin_owner_burn() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | upper_case }}");

    let burned_amount = Amount::new(100);

    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "burn",
                args![burned_amount.clone()],
            )
            .call_method(meme_coin_result.meme_coin_component, "balance", args![])
            .build_and_seal(&meme_coin_result.admin_account_secret),
        vec![meme_coin_result.admin_account_proof],
    );
    assert!(result.finalize.result.is_accept());

    let memecoin_balance = result.finalize.execution_results[1]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(
        memecoin_balance,
        meme_coin_result.initial_supply.sub(burned_amount)
    );
}
