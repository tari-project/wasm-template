#![no_std]

use tari_template_lib::prelude::*;


#[template]
mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        value: u32,
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn new() -> Self {
            Self { value: 0 }
        }

        /// Use this to instantiate the component and call the increase method in one transaction.
        pub fn with_address(alloc: ComponentAddressAllocation) -> Component<Self> {
            Component::new(Self { value: 0 }).with_address_allocation(alloc).create()
        }

        pub fn value(&self) -> u32 {
            self.value
        }

        pub fn increase(&mut self) {
            self.increase_by(1)
        }

        pub fn increase_by(&mut self, value: u32) {
            self.value = self.value.checked_add(value).expect("value overflowed");
        }
    }
}
