use tari_template_test_tooling::transaction::{args, Transaction};
use tari_template_lib::prelude::{Amount, ComponentAddress, call_args};
use tari_template_test_tooling::TemplateTest;

#[test]
fn test_fungible() {
    let mut template_test = TemplateTest::new(vec!["."]);
    let initial_supply = Amount::from(1_000_000_000_000u64);
    let fungible_component: ComponentAddress =
        template_test.call_function("{{ project-name | upper_camel_case }}", "mint", call_args![initial_supply, "TEST"], vec![]);

    let (receiver_address, receiver_proof, receiver_secret_key) = template_test.create_empty_account();

    let result = template_test.execute_expect_success(
        Transaction::builder()
             .call_method(fungible_component, "resource_address", args![])
            .put_last_instruction_output_on_workspace("resource_address")
            .call_method(fungible_component, "take_free_coins", args![100])
            .put_last_instruction_output_on_workspace("receiver_bucket")
            .call_method(receiver_address, "deposit", args![Workspace("receiver_bucket")])
            .call_method(fungible_component, "balance", args![])
            .call_method(receiver_address, "balance", args![Workspace("resource_address")])
            .build_and_seal(&receiver_secret_key),
        vec![receiver_proof],
    );

    for log in result.finalize.logs {
        eprintln!("LOG: {}", log);
    }
    eprintln!("{:?}", result.finalize.execution_results);
    let fungible_account_balance = result.finalize.execution_results[5].decode::<Amount>().unwrap();
    let receiver_balance = result.finalize.execution_results[6].decode::<Amount>().unwrap();
    assert_eq!(
        fungible_account_balance,
        initial_supply - 100.into()
    );
    assert_eq!(receiver_balance, 100);
}
