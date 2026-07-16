//! The plause probe: a minimal, fully deterministic CLAP plugin that exists
//! so plause's integration tests have a plugin they control completely.
//!
//! Milestone 1 scope: a complete, inspectable surface — descriptor, one
//! stereo audio output port, note ports in both directions, and two params
//! (one continuous, one stepped) — with a `process()` that only outputs
//! silence. Deterministic audible behavior arrives in milestone 2
//! (see MILESTONES.md).

use clack_extensions::audio_ports::*;
use clack_extensions::note_ports::*;
use clack_extensions::params::*;
use clack_plugin::events::spaces::CoreEventSpace;
use clack_plugin::prelude::*;
use std::ffi::CStr;
use std::fmt::Write as _;
use std::sync::atomic::{AtomicU32, Ordering};

/// The continuous parameter: output gain, `0.0..=1.0`.
pub const PARAM_GAIN_ID: ClapId = ClapId::new(1);
/// The stepped parameter: a 4-position "mode" switch, `0..=3`.
pub const PARAM_MODE_ID: ClapId = ClapId::new(2);

const DEFAULT_GAIN: f32 = 0.5;
const DEFAULT_MODE: u32 = 0;

pub struct ProbePlugin;

impl Plugin for ProbePlugin {
    type AudioProcessor<'a> = ProbeAudioProcessor<'a>;
    type Shared<'a> = ProbeShared;
    type MainThread<'a> = ProbeMainThread<'a>;

    fn declare_extensions(builder: &mut PluginExtensions<Self>, _shared: Option<&ProbeShared>) {
        builder
            .register::<PluginAudioPorts>()
            .register::<PluginNotePorts>()
            .register::<PluginParams>();
    }
}

impl DefaultPluginFactory for ProbePlugin {
    fn get_descriptor() -> PluginDescriptor {
        use clack_plugin::plugin::features::*;

        PluginDescriptor::new("org.plause.probe", "Plause Probe")
            .with_vendor("plause")
            .with_version("0.1.0")
            .with_description("Deterministic test-subject plugin for the plause CLAP host")
            .with_features([INSTRUMENT, STEREO])
    }

    fn new_shared(_host: HostSharedHandle<'_>) -> Result<Self::Shared<'_>, PluginError> {
        Ok(ProbeShared {
            params: ProbeParams::new(),
        })
    }

    fn new_main_thread<'a>(
        _host: HostMainThreadHandle<'a>,
        shared: &'a Self::Shared<'a>,
    ) -> Result<Self::MainThread<'a>, PluginError> {
        Ok(ProbeMainThread { shared })
    }
}

pub struct ProbeShared {
    params: ProbeParams,
}

impl PluginShared<'_> for ProbeShared {}

pub struct ProbeMainThread<'a> {
    shared: &'a ProbeShared,
}

impl<'a> PluginMainThread<'a, ProbeShared> for ProbeMainThread<'a> {}

pub struct ProbeAudioProcessor<'a> {
    shared: &'a ProbeShared,
}

impl<'a> PluginAudioProcessor<'a, ProbeShared, ProbeMainThread<'a>> for ProbeAudioProcessor<'a> {
    fn activate(
        _host: HostAudioProcessorHandle<'a>,
        _main_thread: &mut ProbeMainThread,
        shared: &'a ProbeShared,
        _audio_config: PluginAudioConfiguration,
    ) -> Result<Self, PluginError> {
        Ok(Self { shared })
    }

    fn process(
        &mut self,
        _process: Process,
        mut audio: Audio,
        events: Events,
    ) -> Result<ProcessStatus, PluginError> {
        for event_batch in events.input.batch() {
            for event in event_batch.events() {
                self.shared.params.handle_event(event);
            }
        }

        // Milestone 1: silence only. The tone generator lands in milestone 2.
        let mut port = audio
            .port_pair(0)
            .ok_or(PluginError::Message("no audio output port"))?;
        let mut channels = port
            .channels()?
            .into_f32()
            .ok_or(PluginError::Message("expected f32 buffers"))?;

        for pair in channels.iter_mut() {
            match pair {
                ChannelPair::OutputOnly(out)
                | ChannelPair::InPlace(out)
                | ChannelPair::InputOutput(_, out) => out.fill(0.0),
                ChannelPair::InputOnly(_) => {}
            }
        }

        Ok(ProcessStatus::Continue)
    }
}

impl PluginAudioPortsImpl for ProbeMainThread<'_> {
    fn count(&mut self, is_input: bool) -> u32 {
        if is_input { 0 } else { 1 }
    }

    fn get(&mut self, index: u32, is_input: bool, writer: &mut AudioPortInfoWriter) {
        if !is_input && index == 0 {
            writer.set(&AudioPortInfo {
                id: ClapId::new(1),
                name: b"main",
                channel_count: 2,
                flags: AudioPortFlags::IS_MAIN,
                port_type: Some(AudioPortType::STEREO),
                in_place_pair: None,
            });
        }
    }
}

impl PluginNotePortsImpl for ProbeMainThread<'_> {
    fn count(&mut self, _is_input: bool) -> u32 {
        1
    }

    fn get(&mut self, index: u32, is_input: bool, writer: &mut NotePortInfoWriter) {
        if index != 0 {
            return;
        }
        writer.set(&NotePortInfo {
            id: ClapId::new(1),
            name: if is_input { b"notes in" } else { b"notes out" },
            supported_dialects: NoteDialects::CLAP,
            preferred_dialect: Some(NoteDialect::Clap),
        });
    }
}

impl PluginMainThreadParams for ProbeMainThread<'_> {
    fn count(&mut self) -> u32 {
        2
    }

    fn get_info(&mut self, param_index: u32, info: &mut ParamInfoWriter) {
        match param_index {
            0 => info.set(&ParamInfo {
                id: PARAM_GAIN_ID,
                flags: ParamInfoFlags::IS_AUTOMATABLE,
                cookie: Default::default(),
                name: b"Gain",
                module: b"",
                min_value: 0.0,
                max_value: 1.0,
                default_value: DEFAULT_GAIN as f64,
            }),
            1 => info.set(&ParamInfo {
                id: PARAM_MODE_ID,
                flags: ParamInfoFlags::IS_AUTOMATABLE | ParamInfoFlags::IS_STEPPED,
                cookie: Default::default(),
                name: b"Mode",
                module: b"",
                min_value: 0.0,
                max_value: 3.0,
                default_value: DEFAULT_MODE as f64,
            }),
            _ => {}
        }
    }

    fn get_value(&mut self, param_id: ClapId) -> Option<f64> {
        match param_id {
            id if id == PARAM_GAIN_ID => Some(self.shared.params.gain() as f64),
            id if id == PARAM_MODE_ID => Some(self.shared.params.mode() as f64),
            _ => None,
        }
    }

    fn value_to_text(
        &mut self,
        param_id: ClapId,
        value: f64,
        writer: &mut ParamDisplayWriter,
    ) -> std::fmt::Result {
        match param_id {
            id if id == PARAM_GAIN_ID => write!(writer, "{value:.2}"),
            id if id == PARAM_MODE_ID => write!(writer, "mode {}", value as u32),
            _ => Err(std::fmt::Error),
        }
    }

    fn text_to_value(&mut self, param_id: ClapId, text: &CStr) -> Option<f64> {
        let text = text.to_str().ok()?;
        match param_id {
            id if id == PARAM_GAIN_ID => text.trim().parse().ok(),
            id if id == PARAM_MODE_ID => text
                .trim()
                .strip_prefix("mode ")
                .unwrap_or(text)
                .parse()
                .ok(),
            _ => None,
        }
    }

    fn flush(&mut self, input_parameter_changes: &InputEvents, _: &mut OutputEvents) {
        for event in input_parameter_changes {
            self.shared.params.handle_event(event);
        }
    }
}

impl PluginAudioProcessorParams for ProbeAudioProcessor<'_> {
    fn flush(&mut self, input_parameter_changes: &InputEvents, _: &mut OutputEvents) {
        for event in input_parameter_changes {
            self.shared.params.handle_event(event);
        }
    }
}

/// Parameter storage shared between the main and audio threads.
struct ProbeParams {
    gain: AtomicU32,
    mode: AtomicU32,
}

impl ProbeParams {
    fn new() -> Self {
        Self {
            gain: AtomicU32::new(DEFAULT_GAIN.to_bits()),
            mode: AtomicU32::new(DEFAULT_MODE),
        }
    }

    fn gain(&self) -> f32 {
        f32::from_bits(self.gain.load(Ordering::Relaxed))
    }

    fn mode(&self) -> u32 {
        self.mode.load(Ordering::Relaxed)
    }

    fn handle_event(&self, event: &UnknownEvent) {
        if let Some(CoreEventSpace::ParamValue(event)) = event.as_core_event() {
            match event.param_id() {
                Some(id) if id == PARAM_GAIN_ID => self.gain.store(
                    (event.value() as f32).clamp(0.0, 1.0).to_bits(),
                    Ordering::Relaxed,
                ),
                Some(id) if id == PARAM_MODE_ID => self
                    .mode
                    .store((event.value() as u32).min(3), Ordering::Relaxed),
                _ => {}
            }
        }
    }
}

clack_export_entry!(SinglePluginEntry<ProbePlugin>);
