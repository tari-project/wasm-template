use tari_template_test_tooling::engine_types::commit_result::RejectReason;
use tari_template_lib::models::{Metadata, NonFungibleAddress};
use tari_template_lib::prelude::{Amount, ComponentAddress};
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::TemplateTest;
use tari_template_test_tooling::transaction::{args, Transaction};

struct CreateMemeCoinResult {
    pub initial_supply: Amount,
    pub admin_account_component: ComponentAddress,
    pub admin_account_proof: NonFungibleAddress,
    pub admin_account_secret: RistrettoSecretKey,
    pub meme_coin_component: ComponentAddress,
}

fn create_meme_coin(test: &mut TemplateTest, name: &str) -> CreateMemeCoinResult {
    let initial_supply = 1_000_000_000_000u64;
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    // create new memecoin
    let memecoin_template_addr = test.get_template_address("{{ project-name | upper_camel_case }}");
    let create_coin_result = test.execute_expect_success(
        Transaction::builder()
            .call_function(
                memecoin_template_addr,
                "create",
                args![
                    initial_supply,
                    name.to_string(),
                    None::<String>,
                    Metadata::new()
                ],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    let coin_component = create_coin_result.finalize.execution_results[0]
        .decode::<ComponentAddress>()
        .unwrap();

    CreateMemeCoinResult {
        initial_supply: initial_supply.into(),
        admin_account_component: account_component,
        admin_account_proof: owner_proof,
        admin_account_secret: account_secret_key,
        meme_coin_component: coin_component,
    }
}

#[test]
fn test_memecoin_owner_only_allowed_method() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | shouty_kebab_case }}");

    // make sure that admin only method is working
    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "burn",
                args![10],
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
                args![10],
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
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | shouty_kebab_case }}");
    let (target_account_addr, _, _) = template_test.create_empty_account();

    let withdraw_amount = 10;

    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "withdraw",
                args![withdraw_amount],
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
        meme_coin_result.initial_supply - withdraw_amount.into()
    );
}

#[test]
fn test_memecoin_owner_burn() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | shouty_kebab_case }}");

    let burned_amount = Amount::from(100);

    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "burn",
                args![burned_amount],
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
        meme_coin_result.initial_supply - burned_amount
    );
}

#[test]
fn test_memecoin_owner_mint() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let meme_coin_result = create_meme_coin(&mut template_test, "{{ project-name | shouty_kebab_case }}");

    let deposited_amount = Amount::from(100);

    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                meme_coin_result.meme_coin_component,
                "mint",
                args![deposited_amount],
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
        meme_coin_result.initial_supply + deposited_amount
    );
}
