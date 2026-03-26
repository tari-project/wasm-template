use tari_template_test_tooling::transaction::{Transaction, args};
use tari_template_test_tooling::support::assert_error::assert_reject_reason;
use tari_template_lib::types::NonFungibleAddress;
use tari_template_lib::prelude::{Amount, ComponentAddress};
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::TemplateTest;

struct AirdropResult {
    _account_address: ComponentAddress,
    _account_proof: NonFungibleAddress,
    _account_secret: RistrettoSecretKey,
    airdrop_address: ComponentAddress,
}

fn airdrop(test: &mut TemplateTest) -> AirdropResult {
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_function(
                test.get_template_address("{{ project-name | upper_camel_case }}"),
                "new",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    let airdrop_address = result.finalize.execution_results[0]
        .decode::<ComponentAddress>()
        .unwrap();

    AirdropResult {
        _account_address: account_component,
        _account_proof: owner_proof,
        _account_secret: account_secret_key,
        airdrop_address,
    }
}

#[test]
fn test_airdrop_add_recipient_airdrop_already_started() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);

    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    let result = test.execute_expect_failure(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "add_recipient",
                args![account_component],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert_reject_reason(result, "Airdrop already started");
}

#[test]
fn test_airdrop_add_recipient_airdrop_allow_list_full() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);

    // open airdrop
    let (_account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());

    // add recipients
    for _ in 0..100 {
        let (account_component, owner_proof, account_secret_key) = test.create_funded_account();
        let result = test.execute_expect_success(
            Transaction::builder_localnet()
                .call_method(
                    airdrop_result.airdrop_address,
                    "add_recipient",
                    args![account_component],
                )
                .build_and_seal(&account_secret_key),
            vec![owner_proof.clone()],
        );
        assert!(result.finalize.result.is_accept());
    }

    // fail to add more recipient than allowed
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let result = test.execute_expect_failure(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "add_recipient",
                args![account_component],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert_reject_reason(result, "Airdrop allow list is full");
}

#[test]
fn test_airdrop_add_recipient_success() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);

    // open airdrop
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "add_recipient",
                args![account_component],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());
}

#[test]
fn test_airdrop_open_airdrop_failure() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);

    // open airdrop
    let (_account_component, owner_proof, account_secret_key) = test.create_funded_account();
    let result = test.execute_expect_failure(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert_reject_reason(result, "Airdrop already open");
}

#[test]
fn test_airdrop_claim_any_success() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    // get claimed count
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "num_claimed",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());
    let claim_count = result.finalize.execution_results[0]
        .decode::<u32>()
        .unwrap();
    assert_eq!(claim_count, 0);

    // get vault balance
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "vault_balance",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());
    let vault_balance = result.finalize.execution_results[0]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(vault_balance, 100);

    // claim
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "add_recipient",
                args![account_component],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "claim_any",
                args![account_component],
            )
            .put_last_instruction_output_on_workspace("airdrop")
            .call_method(account_component, "deposit", args![Workspace("airdrop")])
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());

    // get claimed count again
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "num_claimed",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());
    let claim_count = result.finalize.execution_results[0]
        .decode::<u32>()
        .unwrap();
    assert_eq!(claim_count, 1);

    // get vault balance again
    let result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "vault_balance",
                args![],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );
    assert!(result.finalize.result.is_accept());
    let vault_balance = result.finalize.execution_results[0]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(vault_balance, 99);
}

#[test]
fn test_airdrop_claim_any_airdrop_not_open() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    // claim
    let reject_reason = test.execute_expect_failure(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "claim_any",
                args![account_component],
            )
            .put_last_instruction_output_on_workspace("airdrop")
            .call_method(account_component, "deposit", args![Workspace("airdrop")])
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert_reject_reason(reject_reason, "Airdrop is not open");
}

#[test]
fn test_airdrop_claim_any_already_claimed() {
    let mut test = TemplateTest::my_crate();
    let airdrop_result = airdrop(&mut test);
    let (account_component, owner_proof, account_secret_key) = test.create_funded_account();

    // claim
    let reject_reason = test.execute_expect_failure(
        Transaction::builder_localnet()
            .call_method(
                airdrop_result.airdrop_address,
                "open_airdrop",
                args![],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "add_recipient",
                args![account_component],
            )
            .call_method(
                airdrop_result.airdrop_address,
                "claim_any",
                args![account_component],
            )
            .put_last_instruction_output_on_workspace("airdrop")
            .call_method(account_component, "deposit", args![Workspace("airdrop")])
            .call_method(
                airdrop_result.airdrop_address,
                "claim_any",
                args![account_component],
            )
            .build_and_seal(&account_secret_key),
        vec![owner_proof.clone()],
    );

    assert_reject_reason(reject_reason, "is not in allow list or has already been claimed");
}