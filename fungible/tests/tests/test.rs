#[cfg(test)]
mod test {

    use tari_template_lib::args;
    use tari_template_lib::models::ComponentAddress;
    use tari_template_test_tooling::TemplateTest;

    #[test]
    fn test_fungible() {
        let template_test = TemplateTest::new(vec!["../package"]);
        let component_address: ComponentAddress =
            template_test.call_function("FungibleAccount", "initial_mint", args![Amount(100)]);

        let receiver_address: ComponentAddress =
            template_test.call_method(owner_address, "new_account", args![]);

        let result = template_test.execute(vec![
            Instruction::CallMethod {
                template_address: template_test.get_template_address("FungibleAccount"),
                component_address: owner_address,
                method: "withdraw".to_string(),
                args: args![Amount(100)],
            },
            Instruction::PutLastInstructionOutputOnWorkspace {
                key: b"foo_bucket".to_vec(),
            },
            Instruction::CallMethod {
                template_address: template_test.get_template_address("FungibleAccount"),
                component_address: receiver_address,
                method: "deposit".to_string(),
                args: args![Workspace(b"foo_bucket")],
            },
            Instruction::CallMethod {
                template_address: template_test.get_template_address("FungibleAccount"),
                component_address: owner_address,
                method: "balance".to_string(),
                args: args![],
            },
            Instruction::CallMethod {
                template_address: template_test.get_template_address("FungibleAccount"),
                component_address: receiver_address,
                method: "balance".to_string(),
                args: args![],
            },
        ]);
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
