use tari_template_test_tooling::{TemplateTest, transaction::{args, Transaction}};

#[test]
fn it_works() {
    let mut test = TemplateTest::my_crate();

    let template = test.get_template_address("{{ project-name | upper_camel_case }}");

    let _result = test.execute_expect_success(
        Transaction::builder_localnet()
            .call_function(template, "new", args![])
            // .call_method("addr", "call_something", args![])
            .build_and_seal(test.secret_key()),
        vec![test.owner_proof()]
    );
}
