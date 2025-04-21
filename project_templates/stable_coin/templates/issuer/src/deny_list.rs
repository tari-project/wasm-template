// Copyright 2023 The Tari Project
// SPDX-License-Identifier: BSD-3-Clause

use std::collections::HashMap;
use tari_template_lib::models::ComponentAddress;
use tari_template_lib::prelude::RistrettoPublicKeyBytes;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DenyList {
    id_counter: u64,
    public_keys: HashMap<RistrettoPublicKeyBytes, u64>,
    components: HashMap<ComponentAddress, u64>,
}

impl DenyList {
    pub fn new() -> Self {
        Self {
            id_counter: 0,
            public_keys: HashMap::new(),
            components: HashMap::new(),
        }
    }

    pub fn insert_entry(
        &mut self,
        public_key: RistrettoPublicKeyBytes,
        component: ComponentAddress,
    ) -> bool {
        let id = self.next_id();
        if self.public_keys.contains_key(&public_key) || self.components.contains_key(&component) {
            return false;
        }
        self.public_keys.insert(public_key, id);
        self.components.insert(component, id);
        true
    }

    pub fn contains_public_key(&self, public_key: &RistrettoPublicKeyBytes) -> bool {
        self.public_keys.contains_key(public_key)
    }

    pub fn contains_component(&self, component: &ComponentAddress) -> bool {
        self.components.contains_key(component)
    }

    pub fn remove_by_public_key(
        &mut self,
        public_key: &RistrettoPublicKeyBytes,
    ) -> Option<ComponentAddress> {
        let Some(id) = self.public_keys.remove(public_key) else {
            return None;
        };
        let mut component = None;
        self.components.retain(|addr, v| {
            if *v == id {
                component = Some(*addr);
                false
            } else {
                true
            }
        });
        component
    }

    // pub fn remove_by_component(&mut self, component: &ComponentAddress) -> bool {
    //     let Some(id) = self.components.remove(component) else {
    //         return false;
    //     };
    //     self.public_keys.retain(|_, v| *v != id);
    //     true
    // }

    fn next_id(&mut self) -> u64 {
        let id = self.id_counter;
        self.id_counter += 1;
        id
    }
}
