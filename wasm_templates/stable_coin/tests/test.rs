use tari_template_lib::args;
use tari_template_lib::models::{
    Amount, ComponentAddress, Metadata, NonFungibleAddress, ResourceAddress,
};
use tari_template_test_tooling::crypto::RistrettoSecretKey;
use tari_template_test_tooling::TemplateTest;
use tari_transaction::Transaction;

#[test]
fn it_increases_and_decreases_supply() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        ..
    } = setup();

    let result = test.execute_expect_success(
        Transaction::builder()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "increase_supply", args![123])
            .call_method(stable_coin_component, "total_supply", args![])
            .drop_all_proofs_in_workspace()
            .sign(&admin_key)
            .build(),
        vec![admin_proof.clone()],
    );

    let total_supply = result.finalize.execution_results[3]
        .decode::<Amount>()
        .unwrap();

    assert_eq!(total_supply, Amount(1_000_000_123));

    let result = test.execute_expect_success(
        Transaction::builder()
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(stable_coin_component, "decrease_supply", args![Amount(456)])
            .call_method(stable_coin_component, "total_supply", args![])
            .drop_all_proofs_in_workspace()
            .sign(&admin_key)
            .build(),
        vec![admin_proof],
    );

    let total_supply = result.finalize.execution_results[3]
        .decode::<Amount>()
        .unwrap();

    assert_eq!(total_supply, Amount(1_000_000_123 - 456));
}

#[test]
fn it_allows_users_to_transact() {
    let TestSetup {
        mut test,
        stable_coin_component,
        admin_proof,
        admin_key,
        admin_account,
        admin_badge_resource,
        user_badge_resource,
        token_resource,
        ..
    } = setup();

    let (alice_account, alice_proof, alice_key) = test.create_empty_account();
    let (bob_account, _, _) = test.create_empty_account();

    // Allow Alice to transact and provision funds in her account
    test.execute_expect_success(
        Transaction::builder()
            // Auth
            .create_proof(admin_account, admin_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            // Withdraw for new stable coin customer
            .call_method(
                stable_coin_component,
                "create_new_user",
                args![123, alice_account],
            )
            .put_last_instruction_output_on_workspace("badge")
            .call_method(stable_coin_component, "withdraw", args![Amount(1234)])
            .put_last_instruction_output_on_workspace("funds")
            // Deposit badge and funds into Alice's account
            .call_method(alice_account, "deposit", args![Workspace("badge")])
            .call_method(alice_account, "deposit", args![Workspace("funds")])
            .drop_all_proofs_in_workspace()
            .sign(&admin_key)
            .build(),
        vec![admin_proof.clone()],
    );

    // Alice to Bob should fail (Bob is not allowed to transact)
    let result = test.execute_expect_success(
        Transaction::builder()
            .create_proof(alice_account, user_badge_resource)
            .put_last_instruction_output_on_workspace("proof")
            .call_method(alice_account, "withdraw", args![token_resource, 456])
            .put_last_instruction_output_on_workspace("funds")
            .call_method(bob_account, "deposit", args![Workspace("funds")])
            .call_method(bob_account, "balance", args![token_resource])
            .drop_all_proofs_in_workspace()
            .sign(&alice_key)
            .build(),
        vec![alice_proof.clone()],
    );

    let bob_balance = result.finalize.execution_results[5]
        .decode::<Amount>()
        .unwrap();
    assert_eq!(bob_balance, Amount(456));
}

struct TestSetup {
    test: TemplateTest,
    stable_coin_component: ComponentAddress,
    admin_account: ComponentAddress,
    admin_proof: NonFungibleAddress,
    admin_key: RistrettoSecretKey,
    admin_badge_resource: ResourceAddress,
    user_badge_resource: ResourceAddress,
    token_resource: ResourceAddress,
}

fn setup() -> TestSetup {
    let mut test = TemplateTest::new(["./"]);
    let (admin_account, admin_proof, admin_key) = test.create_owned_account();
    let template = test.get_template_address("TariStableCoin");
    let mut metadata = Metadata::new();
    metadata
        .insert("provider_name", "Stable coinz 4 U")
        .insert("collateralized_by", "Z$")
        .insert("issuing_authority", "Bank of Silly Walks")
        .insert("issued_at", "2023-01-01");

    let result = test.execute_expect_success(
        Transaction::builder()
            .call_function(
                template,
                "instantiate",
                args![1_000_000_000, "SC4U", metadata, true],
            )
            .put_last_instruction_output_on_workspace("admin_badge")
            .call_method(admin_account, "deposit", args![Workspace("admin_badge")])
            .sign(&admin_key)
            .build(),
        vec![admin_proof.clone()],
    );

    let stable_coin_component = result
        .finalize
        .result
        .accept()
        .unwrap()
        .up_iter()
        .find(|(id, s)| {
            id.is_component()
                && s.substate_value().component().unwrap().template_address == template
        })
        .map(|(id, _)| id.as_component_address().unwrap())
        .unwrap();

    let indexed = test
        .read_only_state_store()
        .inspect_component(stable_coin_component)
        .unwrap();

    let token_vault = indexed
        .get_value("$.token_vault")
        .unwrap()
        .expect("user_badge_resource not found");
    let user_badge_resource = indexed
        .get_value("$.user_auth_resource")
        .unwrap()
        .expect("user_auth_resource not found");
    let admin_badge_resource = indexed
        .get_value("$.admin_auth_resource")
        .unwrap()
        .expect("admin_auth_resource not found");

    let vault = test
        .read_only_state_store()
        .get_vault(&token_vault)
        .unwrap();
    let token_resource = *vault.resource_address();

    TestSetup {
        test,
        stable_coin_component,
        admin_account,
        admin_proof,
        admin_key,
        admin_badge_resource,
        user_badge_resource,
        token_resource,
    }
}
