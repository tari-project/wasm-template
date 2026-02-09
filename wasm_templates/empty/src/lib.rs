use tari_template_lib::prelude::*;


#[template]
mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        // Add fields here
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn new() -> Component<Self> {
            Component::new(Self { })
            // TODO: set access rules here as needed
            // .with_access_rules(ComponentAccessRules::new().method("xxx", rule![allow_all]).default(AccessRule::DenyAll))
            .create()
        }

        /// Use this to instantiate the component and call the increase method in one transaction.
        pub fn with_address(alloc: ComponentAddressAllocation) -> Component<Self> {
            Component::new(Self { value: 0 }).with_address_allocation(alloc).create()
        }

        // TODO: add template functions and component methods here
    }
}
