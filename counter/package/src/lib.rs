use tari_template_lib::prelude::*;

#[template]
mod counter {
    pub struct Counter {
        value: u32,
    }

    impl Counter {
        pub fn new() -> Self {
            Self { value: 0 }
        }

        pub fn value(&self) -> u32 {
            self.value
        }

        pub fn increase(&mut self) {
            self.value += 1;
        }
    }
}
