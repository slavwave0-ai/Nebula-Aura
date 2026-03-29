use std::collections::BTreeMap;

use nih_plug::prelude::FloatParam;
use serde::{Deserialize, Serialize};

use crate::NebulaAuraParams;

#[derive(Default)]
pub struct PresetStore {
    presets: BTreeMap<String, String>,
}

impl PresetStore {
    pub fn list(&self) -> Vec<String> {
        self.presets.keys().cloned().collect()
    }

    #[allow(dead_code)]
    pub fn save(&mut self, name: &str, payload: &PresetPayload) {
        if let Ok(json) = serde_json::to_string(payload) {
            self.presets.insert(name.to_string(), json);
        }
    }

    #[allow(dead_code)]
    pub fn load(&self, name: &str) -> Option<PresetPayload> {
        self.presets
            .get(name)
            .and_then(|s| serde_json::from_str::<PresetPayload>(s).ok())
    }
}

#[derive(Serialize, Deserialize, Clone, Default)]
pub struct PresetPayload {
    pub input_db: f32,
    pub output_db: f32,
    pub harmonics: f32,
    pub frequency: f32,
    pub slope: f32,
    pub odd_db: f32,
    pub even_db: f32,
    pub mix: f32,
}

#[derive(Default)]
pub struct MappingStore {
    assignments: BTreeMap<u8, String>,
}

impl MappingStore {
    pub fn apply(&self, cc: u8, value: f32, params: &NebulaAuraParams) {
        if let Some(id) = self.assignments.get(&cc) {
            let normalized = value.clamp(0.0, 1.0);
            if id == "in-level" {
                map_param(&params.input_level, normalized)
            } else if id == "out-level" {
                map_param(&params.output_level, normalized)
            } else if id == "harmonics" {
                map_param(&params.harmonics, normalized)
            } else if id == "frequency" {
                map_param(&params.frequency, normalized)
            } else if id == "slope" {
                map_param(&params.slope, normalized)
            } else if id == "odd" {
                map_param(&params.odd, normalized)
            } else if id == "even" {
                map_param(&params.even, normalized)
            } else if id == "mix" {
                map_param(&params.mix, normalized)
            }
        }
    }
}

fn map_param(param: &FloatParam, normalized: f32) {
    let plain = param.preview_plain(normalized);
    param.set_plain_value(plain);
}
