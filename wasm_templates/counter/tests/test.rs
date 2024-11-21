use tari_template_lib::args;
use tari_template_lib::models::ComponentAddress;
use tari_template_test_tooling::TemplateTest;

#[test]
fn test_increment() {
    let mut template_test = TemplateTest::new(["."]);
    let component_address: ComponentAddress =
        template_test.call_function("{{ project-name | upper_camel_case }}", "new", args![], vec![]);
    let proof = template_test.get_test_proof();
    let value: u32 =
        template_test.call_method(component_address, "value", args![], vec![proof.clone()]);

    assert_eq!(value, 0);

    template_test.call_method::<()>(component_address, "increase", args![], vec![proof.clone()]);
    template_test.call_method::<()>(
        component_address,
        "increase_by",
        args![100],
        vec![proof.clone()],
    );
    let value: u32 = template_test.call_method(component_address, "value", args![], vec![proof]);
    assert_eq!(value, 101);
}
