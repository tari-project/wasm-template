#[cfg(test)]
mod test {
    use tari_engine_types::instruction::Instruction;
    use tari_template_lib::args;
    use tari_template_lib::models::ComponentAddress;
    use tari_template_lib::prelude::Amount;
    use tari_template_test_tooling::TemplateTest;

    #[test]
    fn test_fungible() {
        let template_test = TemplateTest::new(vec!["../package"]);
        let initial_supply = Amount(1_000_000_000_000);
        let owner_address: ComponentAddress =
            template_test.call_function("FungibleAccount", "initial_mint", args![initial_supply]);

        let receiver_address: ComponentAddress =
            template_test.call_method(owner_address, "new_account", args![]);

        let result = template_test.try_execute(vec![
            Instruction::CallMethod {
                component_address: owner_address,
                method: "withdraw".to_string(),
                args: args![Amount(100)],
            },
            Instruction::PutLastInstructionOutputOnWorkspace {
                key: b"foo_bucket".to_vec(),
            },
            Instruction::CallMethod {
                component_address: receiver_address,
                method: "deposit".to_string(),
                args: args![Variable("foo_bucket")],
            },
            Instruction::CallMethod {
                component_address: owner_address,
                method: "balance".to_string(),
                args: args![],
            },
            Instruction::CallMethod {
                component_address: receiver_address,
                method: "balance".to_string(),
                args: args![],
            },
        ]).unwrap();
        for log in result.logs {
            eprintln!("LOG: {}", log);
        }
        eprintln!("{:?}", result.execution_results);
        assert_eq!(
            result.execution_results[3].decode::<Amount>().unwrap(),
            initial_supply - 100
        );
        assert_eq!(result.execution_results[4].decode::<Amount>().unwrap(), 100);
    }
}
