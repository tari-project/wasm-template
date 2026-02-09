use tari_template_lib::prelude::*;

// Use talc as the global allocator
#[global_allocator]
static ALLOCATOR: talc::TalckWasm = unsafe { talc::TalckWasm::new_global() };

#[template]
mod {{ project-name | snake_case }} {
    use super::*;

    pub struct {{ project-name | upper_camel_case }} {
        // Add fields here
    }

    impl {{ project-name | upper_camel_case }} {
        pub fn new() -> Component<Self> {
            // TODO: implement constructor
            // Component::new(Self { }).create()
        }
    }
}
