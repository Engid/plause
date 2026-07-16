//! Loading a `.clap` bundle: dylib loading and `clap_entry` resolution.
//!
//! This module is plause's error-reporting showcase: every way a bundle can
//! fail to load is a distinct [`LoadError`] variant with an actionable,
//! single-sentence message. `plause inspect` prints these directly.

use std::path::{Path, PathBuf};

use clack_host::entry::PluginEntryError;
use clack_host::prelude::*;
use clack_host::utils::ClapVersion;

/// Everything that can go wrong between a path and a usable CLAP entry.
#[derive(Debug, thiserror::Error)]
pub enum LoadError {
    #[error("{path} does not exist")]
    NotFound { path: PathBuf },

    #[error(
        "failed to load {path} as a dynamic library: {source} \
         (wrong architecture, missing dependencies, or no `clap_entry` symbol?)"
    )]
    LibraryLoad {
        path: PathBuf,
        source: PluginEntryError,
    },

    #[error("could not resolve the plugin binary inside the macOS bundle {path}")]
    BundleResolveFailed { path: PathBuf },

    #[error("{path} exposes a `clap_entry` symbol, but it is null")]
    NullEntry { path: PathBuf },

    #[error(
        "{path} was built against an incompatible CLAP version ({plugin_version}); \
         this host speaks CLAP {host_version}"
    )]
    IncompatibleClapVersion {
        path: PathBuf,
        plugin_version: String,
        host_version: String,
    },

    #[error("{path} loaded, but its `clap_entry->init()` returned false")]
    EntryInitFailed { path: PathBuf },

    #[error("failed to load {path}: {source}")]
    Other {
        path: PathBuf,
        source: PluginEntryError,
    },

    #[error("{path} loaded, but does not expose a plugin factory")]
    NoPluginFactory { path: PathBuf },

    #[error("{path} loaded, but its plugin factory lists zero plugins")]
    NoPlugins { path: PathBuf },
}

/// A successfully loaded `.clap` bundle, ready for descriptor enumeration and
/// instantiation.
pub struct LoadedBundle {
    path: PathBuf,
    entry: PluginEntry,
}

impl LoadedBundle {
    /// Load the bundle at `path`.
    ///
    /// `path` may be a `.clap` file (Linux/Windows), a `.clap` bundle
    /// directory (macOS — clack resolves the inner binary), or a bare dynamic
    /// library straight out of `cargo build` — no extension check is imposed,
    /// so dev loops can point at `target/debug/lib*.dylib` directly.
    ///
    /// Note on safety: loading a plugin runs arbitrary code from that library
    /// (static initializers, `clap_entry->init()`). That is inherent to being
    /// a plugin host; there is no way to do this "safely" beyond trusting the
    /// file, which is why this call is quarantined here rather than spread
    /// through the codebase.
    pub fn load(path: &Path) -> Result<Self, LoadError> {
        if !path.exists() {
            return Err(LoadError::NotFound {
                path: path.to_path_buf(),
            });
        }

        // SAFETY: see doc comment — inherent to hosting; the entry is kept
        // alive by `LoadedBundle` for as long as anything can reference it.
        let entry = unsafe { PluginEntry::load(path) }.map_err(|e| {
            let path = path.to_path_buf();
            match e {
                PluginEntryError::LibraryLoadingError(_) => {
                    LoadError::LibraryLoad { path, source: e }
                }
                PluginEntryError::ResolveFailed => LoadError::BundleResolveFailed { path },
                PluginEntryError::NullEntryPointer => LoadError::NullEntry { path },
                PluginEntryError::IncompatibleClapVersion { plugin_version } => {
                    LoadError::IncompatibleClapVersion {
                        path,
                        plugin_version: plugin_version.to_string(),
                        host_version: ClapVersion::CURRENT.to_string(),
                    }
                }
                PluginEntryError::EntryInitFailed => LoadError::EntryInitFailed { path },
                other => LoadError::Other {
                    path,
                    source: other,
                },
            }
        })?;

        Ok(LoadedBundle {
            path: path.to_path_buf(),
            entry,
        })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn entry(&self) -> &PluginEntry {
        &self.entry
    }

    /// The bundle's plugin factory, or the appropriate error if it is missing
    /// or empty.
    pub fn plugin_factory(
        &self,
    ) -> Result<clack_host::factory::plugin::PluginFactory<'_>, LoadError> {
        let factory =
            self.entry
                .get_plugin_factory()
                .ok_or_else(|| LoadError::NoPluginFactory {
                    path: self.path.clone(),
                })?;

        if factory.plugin_count() == 0 {
            return Err(LoadError::NoPlugins {
                path: self.path.clone(),
            });
        }

        Ok(factory)
    }
}
