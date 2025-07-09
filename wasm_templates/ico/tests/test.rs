use tari_template_test_tooling::engine_types::commit_result::RejectReason;
use tari_template_lib::models::{Bucket, ComponentAddress, NonFungibleAddress};
use tari_template_lib::types::Amount;
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::transaction::{args, Transaction};
use tari_template_test_tooling::TemplateTest;
use tari_template_lib::constants::XTR;

struct IcoCreateResult {
    pub account_address: ComponentAddress,
    pub account_proof: NonFungibleAddress,
    pub account_secret: RistrettoSecretKey,
    ico_address: ComponentAddress,
}

fn ico(test: &mut TemplateTest) -> IcoCreateResult {
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let create_coin_result = test.execute_expect_success(
        Transaction::builder()
            .call_function(
                test.get_template_address("{{ project-name | upper_camel_case }}Ico"),
                "new",
                args!["{{ project-name | shouty_kebab_case }}-ICO".to_string(), 1_000_000_000, 10],
            )
            .put_last_instruction_output_on_workspace("ret")
            .call_method(
                account_component,
                "deposit",
                args![Workspace("ret.1")],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    let (ico_address, _) = create_coin_result.finalize.execution_results[0]
        .decode::<(ComponentAddress, Bucket)>()
        .unwrap();

    IcoCreateResult {
        account_address: account_component,
        account_proof: owner_proof,
        account_secret: account_secret_key,
        ico_address,
    }
}

#[test]
fn test_buy_success() {
    let mut template_test = TemplateTest::new(["."]);
    let ico_result = ico(&mut template_test);

    // create a non-owner new account
    let (account_component, owner_proof, account_secret_key) = template_test.create_funded_account();

    // buy ICOs with XTR
    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                ico_result.ico_address,
                "ico_resource_address",
                args![],
            )
            .put_last_instruction_output_on_workspace("ico_resource_address")
            .call_method(
                ico_result.ico_address,
                "xtr_balance",
                args![],
            )
            .call_method(
                account_component,
                "balance",
                args![Workspace("ico_resource_address")],
            )
            .call_method(
                account_component,
                "withdraw",
                args![XTR, 100],
            )
            .put_last_instruction_output_on_workspace("xtr_coins")
            .call_method(
                ico_result.ico_address,
                "buy",
                args![Workspace("xtr_coins")],
            )
            .put_last_instruction_output_on_workspace("ico")
            .call_method(
                account_component,
                "deposit",
                args![Workspace("ico")],
            )
            .call_method(
                ico_result.ico_address,
                "xtr_balance",
                args![],
            )
            .call_method(
                account_component,
                "balance",
                args![Workspace("ico_resource_address")],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    let ico_initial_xtr_balance = result.finalize.execution_results[2]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(ico_initial_xtr_balance, Amount::zero());

    let account_initial_ico_balance = result.finalize.execution_results[3]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(account_initial_ico_balance, Amount::zero());

    let ico_final_xtr_balance = result.finalize.execution_results[9]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(ico_final_xtr_balance, 100);

    let account_final_ico_balance = result.finalize.execution_results[10]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(account_final_ico_balance, 10);
}

#[test]
fn test_buy_insufficient_funds() {
    let mut template_test = TemplateTest::new(["."]);
    let ico_result = ico(&mut template_test);

    // create a non-owner new account
    let (account_component, owner_proof, account_secret_key) = template_test.create_funded_account();

    // buy ICOs with XTR
    let reject_reason = template_test.execute_expect_failure(
        Transaction::builder()
            .call_method(
                ico_result.ico_address,
                "ico_resource_address",
                args![],
            )
            .put_last_instruction_output_on_workspace("ico_resource_address")
            .call_method(
                ico_result.ico_address,
                "xtr_balance",
                args![],
            )
            .call_method(
                account_component,
                "balance",
                args![Workspace("ico_resource_address")],
            )
            .call_method(
                account_component,
                "withdraw",
                args![XTR, Amount(5)],
            )
            .put_last_instruction_output_on_workspace("xtr_coins")
            .call_method(
                ico_result.ico_address,
                "buy",
                args![Workspace("xtr_coins")],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert!(matches!(reject_reason, RejectReason::ExecutionFailure(_)));
    if let RejectReason::ExecutionFailure(reason) = reject_reason {
        assert_eq!(reason, "Panic! Insufficient funds! You need more XTR to buy ICOs.");
    }
}

#[test]
fn test_withdraw_access_denied() {
    let mut template_test = TemplateTest::new(["."]);
    let ico_result = ico(&mut template_test);

    // create a non-owner new account
    let (account_component, owner_proof, account_secret_key) = template_test.create_funded_account();

    // buy ICOs with XTR
    let _ = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                account_component,
                "withdraw",
                args![XTR, Amount(100)],
            )
            .put_last_instruction_output_on_workspace("xtr_coins")
            .call_method(
                ico_result.ico_address,
                "buy",
                args![Workspace("xtr_coins")],
            )
            .put_last_instruction_output_on_workspace("ico")
            .call_method(
                account_component,
                "deposit",
                args![Workspace("ico")],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    // try to withdraw funds from ICO using non-owner account
    let reject_reason = template_test.execute_expect_failure(
        Transaction::builder()
            .call_method(
                ico_result.ico_address,
                "withdraw",
                args![Amount(100)],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert!(matches!(reject_reason, RejectReason::ExecutionFailure(_)));
    if let RejectReason::ExecutionFailure(reason) = reject_reason {
        assert!(reason.starts_with("Access Denied:"));
        assert!(reason.contains(
            format!("call component method 'withdraw' on {}", ico_result.ico_address).as_str()
        ));
    }
}

#[test]
fn test_owner_withdraw() {
    let mut template_test = TemplateTest::new(["."]);
    let ico_result = ico(&mut template_test);

    // create a non-owner new account
    let (account_component, owner_proof, account_secret_key) = template_test.create_funded_account();

    // buy ICOs with XTR
    let _ = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                account_component,
                "withdraw",
                args![XTR, Amount(100)],
            )
            .put_last_instruction_output_on_workspace("xtr_coins")
            .call_method(
                ico_result.ico_address,
                "buy",
                args![Workspace("xtr_coins")],
            )
            .put_last_instruction_output_on_workspace("ico")
            .call_method(
                account_component,
                "deposit",
                args![Workspace("ico")],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    // withdraw funds with owner
    let result = template_test.execute_expect_success(
        Transaction::builder()
            .call_method(
                account_component,
                "balance",
                args![XTR],
            )
            .call_method(
                ico_result.ico_address,
                "withdraw",
                args![Amount(100)],
            )
            .put_last_instruction_output_on_workspace("xtr_coins")
            .call_method(
                account_component,
                "deposit",
                args![Workspace("xtr_coins")],
            )
            .call_method(
                account_component,
                "balance",
                args![XTR],
            )
            .build_and_seal(&ico_result.account_secret),
        vec![ico_result.account_proof.clone()],
    );

    let owner_initial_xtr_balance = result.finalize.execution_results[0]
        .decode::<Amount>()
        .unwrap();

    let owner_final_xtr_balance = result.finalize.execution_results[4]
        .decode::<Amount>()
        .unwrap();

    assert_eq!(owner_initial_xtr_balance, owner_final_xtr_balance - 100.into());
}
