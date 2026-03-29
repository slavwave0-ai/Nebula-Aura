mod dsp;
mod editor;
mod preset;

use std::sync::{Arc, Mutex};

use atomic_float::AtomicF32;
use dsp::{ExciterCore, SpectrumFrame};
use nih_plug::prelude::*;
use nih_plug_egui::EguiState;
use preset::{MappingStore, PresetStore};

const MAX_UNDO_STEPS: usize = 50;

#[derive(Params)]
pub struct NebulaAuraParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,

    #[id = "oversampling"]
    pub oversampling: EnumParam<OversamplingMode>,

    #[id = "phase"]
    pub phase_flip: BoolParam,

    #[id = "fx-bypass"]
    pub fx_bypass: BoolParam,

    #[id = "in-level"]
    pub input_level: FloatParam,
    #[id = "out-level"]
    pub output_level: FloatParam,
    #[id = "harmonics"]
    pub harmonics: FloatParam,
    #[id = "frequency"]
    pub frequency: FloatParam,
    #[id = "slope"]
    pub slope: FloatParam,
    #[id = "odd"]
    pub odd: FloatParam,
    #[id = "even"]
    pub even: FloatParam,
    #[id = "mix"]
    pub mix: FloatParam,
}

impl Default for NebulaAuraParams {
    fn default() -> Self {
        let editor_state = EguiState::from_size(1280, 820);
        Self {
            editor_state: Arc::new(editor_state),
            oversampling: EnumParam::new("Oversampling", OversamplingMode::Off),
            phase_flip: BoolParam::new("Phase", false),
            fx_bypass: BoolParam::new("FX Bypass", false),
            input_level: FloatParam::new(
                "Input",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_unit(" dB"),
            output_level: FloatParam::new(
                "Output",
                0.0,
                FloatRange::Linear {
                    min: -50.0,
                    max: 50.0,
                },
            )
            .with_unit(" dB"),
            harmonics: FloatParam::new(
                "Harmonics",
                25.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
            frequency: FloatParam::new(
                "Frequency",
                4000.0,
                FloatRange::Skewed {
                    min: 20.0,
                    max: 20_000.0,
                    factor: FloatRange::skew_factor(-2.0),
                },
            )
            .with_unit(" Hz"),
            slope: FloatParam::new(
                "Slope",
                24.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" dB/oct"),
            odd: FloatParam::new(
                "Odd",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            even: FloatParam::new(
                "Even",
                2.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 10.0,
                },
            )
            .with_unit(" dB"),
            mix: FloatParam::new(
                "Mix",
                100.0,
                FloatRange::Linear {
                    min: 0.0,
                    max: 100.0,
                },
            )
            .with_unit(" %"),
        }
    }
}

#[derive(Enum, Clone, Copy, PartialEq, Eq, Debug)]
pub enum OversamplingMode {
    Off,
    X2,
    X4,
    X6,
    X8,
}

impl OversamplingMode {
    pub fn factor(self) -> usize {
        match self {
            Self::Off => 1,
            Self::X2 => 2,
            Self::X4 => 4,
            Self::X6 => 6,
            Self::X8 => 8,
        }
    }
}

pub struct NebulaAura {
    params: Arc<NebulaAuraParams>,
    core: ExciterCore,
    spectrum: Arc<Mutex<SpectrumFrame>>,
    undo: Vec<dsp::Snapshot>,
    redo: Vec<dsp::Snapshot>,
    slot_a: dsp::Snapshot,
    slot_b: dsp::Snapshot,
    ab_is_a: bool,
    preset_store: PresetStore,
    midi_mapping: MappingStore,
    midi_enabled: bool,
    current_gain_reduction: Arc<AtomicF32>,
}

impl Default for NebulaAura {
    fn default() -> Self {
        let params = Arc::new(NebulaAuraParams::default());
        let spectrum = Arc::new(Mutex::new(SpectrumFrame::default()));
        Self {
            core: ExciterCore::default(),
            spectrum,
            undo: Vec::with_capacity(MAX_UNDO_STEPS),
            redo: Vec::with_capacity(MAX_UNDO_STEPS),
            slot_a: dsp::Snapshot::default(),
            slot_b: dsp::Snapshot::default(),
            ab_is_a: true,
            preset_store: PresetStore::default(),
            midi_mapping: MappingStore::default(),
            midi_enabled: true,
            current_gain_reduction: Arc::new(AtomicF32::new(0.0)),
            params,
        }
    }
}

impl Plugin for NebulaAura {
    const NAME: &'static str = "Nebula Aura";
    const VENDOR: &'static str = "Nebula Audio";
    const URL: &'static str = "";
    const EMAIL: &'static str = "support@nebula.audio";

    const VERSION: &'static str = "1.0.0";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout::const_default(),
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(1),
            main_output_channels: NonZeroU32::new(1),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        Some(editor::create_editor(
            self.params.clone(),
            self.spectrum.clone(),
            self.current_gain_reduction.clone(),
            async_executor,
            self.ab_is_a,
            self.midi_enabled,
            self.preset_store.list(),
        ))
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.core.reset(
            buffer_config.sample_rate as f32,
            buffer_config.max_buffer_size as usize,
        );
        true
    }

    fn reset(&mut self) {
        self.core.clear();
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let snapshot = self.snapshot_params();
        self.core.set_snapshot(snapshot.clone());

        if self.params.fx_bypass.value() {
            return ProcessStatus::Normal;
        }

        let mut events = context.next_event();
        while let Some(event) = events {
            if let NoteEvent::MidiCC { cc, value, .. } = event {
                if self.midi_enabled {
                    self.midi_mapping.apply(cc, value, &self.params);
                }
            }
            events = context.next_event();
        }

        self.core.process(buffer, self.params.phase_flip.value());
        self.current_gain_reduction.store(
            self.core.current_gain_reduction_db(),
            std::sync::atomic::Ordering::Relaxed,
        );

        if let Ok(mut frame) = self.spectrum.lock() {
            *frame = self.core.latest_spectrum();
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for NebulaAura {
    const CLAP_ID: &'static str = "com.nebulaaudio.nebulaaura";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("64-bit spectral exciter");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::AudioEffect, ClapFeature::Stereo];
}

impl Vst3Plugin for NebulaAura {
    const VST3_CLASS_ID: [u8; 16] = *b"NebulaAuraVST310";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Tools];
}

impl NebulaAura {
    fn snapshot_params(&self) -> dsp::Snapshot {
        dsp::Snapshot {
            input_db: self.params.input_level.value(),
            output_db: self.params.output_level.value(),
            harmonics: self.params.harmonics.value() / 100.0,
            frequency: self.params.frequency.value(),
            slope: self.params.slope.value(),
            odd_db: self.params.odd.value(),
            even_db: self.params.even.value(),
            mix: self.params.mix.value() / 100.0,
            oversampling: self.params.oversampling.value().factor(),
        }
    }

    #[allow(dead_code)]
    fn push_undo(&mut self) {
        self.undo.push(self.snapshot_params());
        if self.undo.len() > MAX_UNDO_STEPS {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    #[allow(dead_code)]
    fn toggle_ab(&mut self) {
        let current = self.snapshot_params();
        if self.ab_is_a {
            self.slot_a = current;
            self.apply_snapshot(self.slot_b.clone());
        } else {
            self.slot_b = current;
            self.apply_snapshot(self.slot_a.clone());
        }
        self.ab_is_a = !self.ab_is_a;
    }

    fn apply_snapshot(&self, s: dsp::Snapshot) {
        self.params.input_level.set_plain_value(s.input_db);
        self.params.output_level.set_plain_value(s.output_db);
        self.params.harmonics.set_plain_value(s.harmonics * 100.0);
        self.params.frequency.set_plain_value(s.frequency);
        self.params.slope.set_plain_value(s.slope);
        self.params.odd.set_plain_value(s.odd_db);
        self.params.even.set_plain_value(s.even_db);
        self.params.mix.set_plain_value(s.mix * 100.0);
    }
}

nih_export_clap!(NebulaAura);
nih_export_vst3!(NebulaAura);
