use tari_template_lib::args;
use tari_template_lib::prelude::{Amount, ComponentAddress, ResourceAddress};
use tari_template_test_tooling::TemplateTest;
use tari_transaction::TransactionBuilder;

#[test]
fn test_fungible() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let initial_supply = Amount(1_000_000_000_000);
    let fungible_account: ComponentAddress =
        template_test.call_function("{{ project-name | upper_camel_case }}", "mint", args![initial_supply, "TEST".to_string()], vec![]);
    let fungible_account_vault: ResourceAddress = template_test.call_method(fungible_account, "vault_address", args![], vec![]);

    let (receiver_address, receiver_proof, receiver_secret_key) = template_test.create_empty_account();

    let result = template_test.try_execute(
        TransactionBuilder::new()
            .call_method(
                fungible_account,
                "take_free_coins",
                args![Amount(100)],
            )
            .put_last_instruction_output_on_workspace(
                b"receiver_bucket"
            )
            .call_method(
                receiver_address,
                "deposit",
                args![Variable("receiver_bucket")],
            )
            .call_method(
                fungible_account,
                "balance",
                args![],
            )
            .call_method(
                receiver_address,
                "balance",
                args![fungible_account_vault],
            )
            .sign(&receiver_secret_key)
            .build(),
        vec![receiver_proof],
    ).unwrap();
    for log in result.finalize.logs {
        eprintln!("LOG: {}", log);
    }
    eprintln!("{:?}", result.finalize.execution_results);
    let fungible_account_balance = result.finalize.execution_results[3].decode::<Amount>().unwrap();
    let receiver_balance = result.finalize.execution_results[4].decode::<Amount>().unwrap();
    assert_eq!(
        fungible_account_balance,
        initial_supply - 100
    );
    assert_eq!(receiver_balance, 100);
}
