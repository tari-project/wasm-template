use tari_template_test_tooling::{TemplateTest, transaction::{args, Transaction}};

#[test]
fn it_works() {
    let mut test = TemplateTest::new(["."]);

    let counter_template = test.get_template_address("{{ project-name | upper_camel_case }}");

    let result = test.execute_expect_success(
        Transaction::builder()
            .allocate_component_address("addr")
            .call_function(counter_template, "with_address", args![Workspace("addr")])
            .call_method("addr", "value", args![])
            .call_method("addr", "increase", args![])
            .call_method("addr", "increase_by", args![100])
            .call_method("addr", "value", args![])
            .build_and_seal(test.secret_key()),
        vec![test.owner_proof()]
    );

    let v =     result.finalize.execution_results[2].decode::<u32>().unwrap();
    assert_eq!(v, 0);
    let v =     result.finalize.execution_results[5].decode::<u32>().unwrap();
    assert_eq!(v, 101);
}
