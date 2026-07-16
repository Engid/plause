//! Plugin instantiation and inspection: everything `plause inspect` reports.
//!
//! The output type ([`BundleInfo`]) is serializable, and its JSON shape is a
//! public contract — plugin CI pipelines parse it. Fields may be added, but
//! existing fields never change meaning or type.

use std::ffi::CStr;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;

use clack_extensions::audio_ports::{AudioPortInfoBuffer, PluginAudioPorts};
use clack_extensions::gui::PluginGui;
use clack_extensions::note_ports::{
    NoteDialect, NoteDialects, NotePortInfoBuffer, PluginNotePorts,
};
use clack_extensions::params::{ParamInfoBuffer, PluginParams};
use clack_extensions::state::PluginState;
use clack_host::prelude::*;
use serde::Serialize;

use crate::loader::{LoadError, LoadedBundle};

/// Errors specific to instantiating/querying a plugin (on top of loading).
#[derive(Debug, thiserror::Error)]
pub enum InspectError {
    #[error(transparent)]
    Load(#[from] LoadError),

    #[error("failed to instantiate plugin '{id}': {source}")]
    InstantiationFailed {
        id: String,
        source: clack_host::plugin::PluginInstanceError,
    },

    #[error("plugin '{id}' panicked while being inspected — this is a bug in the plugin")]
    PluginPanicked { id: String },
}

/// Inspection report for a whole `.clap` bundle.
#[derive(Debug, Serialize)]
pub struct BundleInfo {
    pub path: String,
    pub plugins: Vec<PluginInfo>,
}

/// Inspection report for one plugin inside a bundle.
#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub descriptor: DescriptorInfo,
    /// CLAP extension ids the plugin exposes, out of the standard extensions
    /// plause knows how to query.
    pub extensions: Vec<String>,
    pub audio_ports: PortDirections<AudioPortDetails>,
    pub note_ports: PortDirections<NotePortDetails>,
    pub params: Vec<ParamDetails>,
}

#[derive(Debug, Serialize)]
pub struct DescriptorInfo {
    pub id: String,
    pub name: Option<String>,
    pub vendor: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub features: Vec<String>,
}

#[derive(Debug, Serialize)]
pub struct PortDirections<T> {
    pub inputs: Vec<T>,
    pub outputs: Vec<T>,
}

#[derive(Debug, Serialize)]
pub struct AudioPortDetails {
    pub id: u32,
    pub name: String,
    pub channel_count: u32,
    pub is_main: bool,
    pub port_type: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NotePortDetails {
    pub id: u32,
    pub name: String,
    pub supported_dialects: Vec<String>,
    pub preferred_dialect: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ParamDetails {
    pub id: u32,
    pub name: String,
    pub module: String,
    pub min_value: f64,
    pub max_value: f64,
    pub default_value: f64,
    pub flags: Vec<String>,
}

/// Load the bundle at `path` and inspect every plugin it contains.
pub fn inspect(path: &Path) -> Result<BundleInfo, InspectError> {
    let bundle = LoadedBundle::load(path)?;
    let factory = bundle.plugin_factory()?;

    let mut plugins = Vec::new();
    for descriptor in factory.plugin_descriptors() {
        let Some(id) = descriptor.id() else {
            continue; // A plugin without an id is unaddressable; skip it.
        };
        let descriptor_info = DescriptorInfo::from_clack(descriptor);

        let id_owned = id.to_owned();
        let result = catch_unwind(AssertUnwindSafe(|| {
            inspect_plugin(&bundle, &id_owned, descriptor_info)
        }))
        .map_err(|_| InspectError::PluginPanicked {
            id: id.to_string_lossy().into_owned(),
        })??;

        plugins.push(result);
    }

    Ok(BundleInfo {
        path: path.display().to_string(),
        plugins,
    })
}

fn inspect_plugin(
    bundle: &LoadedBundle,
    id: &CStr,
    descriptor: DescriptorInfo,
) -> Result<PluginInfo, InspectError> {
    let host_info = HostInfo::new(
        "plause",
        "plause",
        "https://github.com/Engid/plause",
        env!("CARGO_PKG_VERSION"),
    )
    .expect("plause host info contains no null bytes");

    let mut instance = PluginInstance::<PlauseInspector>::new(
        |_| InspectorShared,
        |_| InspectorMainThread,
        bundle.entry(),
        id,
        &host_info,
    )
    .map_err(|source| InspectError::InstantiationFailed {
        id: id.to_string_lossy().into_owned(),
        source,
    })?;

    let mut handle = instance.plugin_handle();

    let mut extensions = Vec::new();
    let audio_ports_ext = handle.get_extension::<PluginAudioPorts>();
    let note_ports_ext = handle.get_extension::<PluginNotePorts>();
    let params_ext = handle.get_extension::<PluginParams>();
    if audio_ports_ext.is_some() {
        extensions.push("clap.audio-ports".to_string());
    }
    if note_ports_ext.is_some() {
        extensions.push("clap.note-ports".to_string());
    }
    if params_ext.is_some() {
        extensions.push("clap.params".to_string());
    }
    if handle.get_extension::<PluginState>().is_some() {
        extensions.push("clap.state".to_string());
    }
    if handle.get_extension::<PluginGui>().is_some() {
        extensions.push("clap.gui".to_string());
    }

    let audio_ports = match &audio_ports_ext {
        Some(ext) => PortDirections {
            inputs: read_audio_ports(ext, &mut handle, true),
            outputs: read_audio_ports(ext, &mut handle, false),
        },
        None => PortDirections {
            inputs: Vec::new(),
            outputs: Vec::new(),
        },
    };

    let note_ports = match &note_ports_ext {
        Some(ext) => PortDirections {
            inputs: read_note_ports(ext, &mut handle, true),
            outputs: read_note_ports(ext, &mut handle, false),
        },
        None => PortDirections {
            inputs: Vec::new(),
            outputs: Vec::new(),
        },
    };

    let params = match &params_ext {
        Some(ext) => read_params(ext, &mut handle),
        None => Vec::new(),
    };

    Ok(PluginInfo {
        descriptor,
        extensions,
        audio_ports,
        note_ports,
        params,
    })
}

fn read_audio_ports(
    ext: &PluginAudioPorts,
    handle: &mut PluginMainThreadHandle<'_>,
    is_input: bool,
) -> Vec<AudioPortDetails> {
    let count = ext.count(handle, is_input);
    let mut buffer = AudioPortInfoBuffer::new();
    (0..count)
        .filter_map(|index| {
            let info = ext.get(handle, index, is_input, &mut buffer)?;
            Some(AudioPortDetails {
                id: info.id.into(),
                name: String::from_utf8_lossy(info.name).into_owned(),
                channel_count: info.channel_count,
                is_main: info
                    .flags
                    .contains(clack_extensions::audio_ports::AudioPortFlags::IS_MAIN),
                port_type: info.port_type.map(|t| t.0.to_string_lossy().into_owned()),
            })
        })
        .collect()
}

fn read_note_ports(
    ext: &PluginNotePorts,
    handle: &mut PluginMainThreadHandle<'_>,
    is_input: bool,
) -> Vec<NotePortDetails> {
    let count = ext.count(handle, is_input);
    let mut buffer = NotePortInfoBuffer::new();
    (0..count)
        .filter_map(|index| {
            let info = ext.get(handle, index, is_input, &mut buffer)?;
            Some(NotePortDetails {
                id: info.id.into(),
                name: String::from_utf8_lossy(info.name).into_owned(),
                supported_dialects: dialect_names(info.supported_dialects),
                preferred_dialect: info.preferred_dialect.map(|d| dialect_name(d).to_string()),
            })
        })
        .collect()
}

fn read_params(ext: &PluginParams, handle: &mut PluginMainThreadHandle<'_>) -> Vec<ParamDetails> {
    let count = ext.count(handle);
    let mut buffer = ParamInfoBuffer::new();
    (0..count)
        .filter_map(|index| {
            let info = ext.get_info(handle, index, &mut buffer)?;
            Some(ParamDetails {
                id: info.id.into(),
                name: String::from_utf8_lossy(info.name).into_owned(),
                module: String::from_utf8_lossy(info.module).into_owned(),
                min_value: info.min_value,
                max_value: info.max_value,
                default_value: info.default_value,
                flags: info
                    .flags
                    .iter_names()
                    .map(|(name, _)| {
                        name.trim_start_matches("IS_")
                            .to_ascii_lowercase()
                            .replace('_', "-")
                    })
                    .collect(),
            })
        })
        .collect()
}

fn dialect_names(dialects: NoteDialects) -> Vec<String> {
    [
        (NoteDialects::CLAP, "clap"),
        (NoteDialects::MIDI, "midi"),
        (NoteDialects::MIDI_MPE, "midi-mpe"),
        (NoteDialects::MIDI2, "midi2"),
    ]
    .into_iter()
    .filter(|(flag, _)| dialects.contains(*flag))
    .map(|(_, name)| name.to_string())
    .collect()
}

fn dialect_name(dialect: NoteDialect) -> &'static str {
    match dialect {
        NoteDialect::Clap => "clap",
        NoteDialect::Midi => "midi",
        NoteDialect::MidiMpe => "midi-mpe",
        NoteDialect::Midi2 => "midi2",
    }
}

impl DescriptorInfo {
    fn from_clack(descriptor: &clack_host::plugin::PluginDescriptor) -> Self {
        let owned = |s: Option<&CStr>| s.map(|s| s.to_string_lossy().into_owned());
        DescriptorInfo {
            id: descriptor
                .id()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default(),
            name: owned(descriptor.name()),
            vendor: owned(descriptor.vendor()),
            version: owned(descriptor.version()),
            description: owned(descriptor.description()),
            features: descriptor
                .features()
                .map(|f| f.to_string_lossy().into_owned())
                .collect(),
        }
    }
}

// --- Minimal host handlers: inspection needs no callbacks. -----------------

struct PlauseInspector;
struct InspectorShared;
struct InspectorMainThread;
struct InspectorAudioProcessor;

impl HostHandlers for PlauseInspector {
    type Shared<'a> = InspectorShared;
    type MainThread<'a> = InspectorMainThread;
    type AudioProcessor<'a> = InspectorAudioProcessor;
}

impl SharedHandler<'_> for InspectorShared {
    // Inspection never activates the plugin, so these requests are no-ops —
    // but they must not panic: plugins may call them at any time.
    fn request_restart(&self) {}
    fn request_process(&self) {}
    fn request_callback(&self) {}
}

impl MainThreadHandler<'_> for InspectorMainThread {}
impl AudioProcessorHandler<'_> for InspectorAudioProcessor {}
