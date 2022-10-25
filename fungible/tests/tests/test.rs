#[cfg(test)]
mod test {

    use tari_dan_engine::tooling::TemplateTest;
    use tari_template_lib::args;
    use tari_template_lib::models::ComponentAddress;

    #[test]
    fn test_increment() {
        let template_test = TemplateTest::new(vec!["../package"]);
        let component_address: ComponentAddress =
            template_test.call_function("Counter", "new", args![]);
        let value: u32 = template_test.call_method(component_address, "value", args![]);

        assert_eq!(value, 0);

        template_test.call_method::<()>(component_address, "increase", args![]);
        let value: u32 = template_test.call_method(component_address, "value", args![]);
        assert_eq!(value, 1);
    }
}
