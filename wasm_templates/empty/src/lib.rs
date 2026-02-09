use tari_template_lib::prelude::*;


#[template]
mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        // Add fields here
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn new() -> Component<Self> {
            Component::new(Self { }).create()
        }

        /// Use this to instantiate the component with a specific address allocation
        pub fn with_address(alloc: ComponentAddressAllocation) -> Component<Self> {
            Component::new(Self { }).with_address_allocation(alloc).create()
        }

        // TODO: add template functions and component methods here
    }
}
