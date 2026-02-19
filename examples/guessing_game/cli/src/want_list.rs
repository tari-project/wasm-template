use ootle_rs::provider::WantInput;
use std::collections::HashSet;
use tari_ootle_common_types::engine_types::substate::SubstateId;
use tari_template_lib_types::{ComponentAddress, ResourceAddress};

#[derive(Debug, Clone, Default)]
pub struct WantList {
    items: HashSet<WantInput>,
}

impl WantList {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_vault_for_resource(
        mut self,
        component_address: ComponentAddress,
        resource_address: ResourceAddress,
        required: bool,
    ) -> Self {
        self.items.insert(WantInput::VaultForResource {
            component_address,
            resource_address,
            required,
        });
        self
    }

    pub fn add_specific_substate<T: Into<SubstateId>>(
        mut self,
        substate_id: T,
        required: bool,
    ) -> Self {
        self.items.insert(WantInput::SpecificSubstate {
            substate_id: substate_id.into(),
            required,
        });
        self
    }

    pub fn items(&self) -> &HashSet<WantInput> {
        &self.items
    }
}
