//! Integration tests for loader + instance, using the plause-probe plugin as
//! a hermetic test subject: these tests exercise the real CLAP ABI boundary
//! (dylib load, `clap_entry`, factory, instantiation, extension queries)
//! without depending on any externally installed plugin.

use std::path::PathBuf;
use std::process::Command;
use std::sync::OnceLock;

use plause_host::instance::{InspectError, inspect};
use plause_host::loader::LoadError;

/// Build plause-probe (cached by cargo; a no-op after the first run) and
/// return the path to the resulting cdylib.
fn probe_path() -> PathBuf {
    static PROBE: OnceLock<PathBuf> = OnceLock::new();
    PROBE
        .get_or_init(|| {
            let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
            let workspace = manifest.parent().unwrap().parent().unwrap();

            let status = Command::new(env!("CARGO"))
                .args(["build", "-p", "plause-probe"])
                .current_dir(workspace)
                .status()
                .expect("failed to invoke cargo to build plause-probe");
            assert!(status.success(), "building plause-probe failed");

            let target_dir = std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|| workspace.join("target"));

            let file_name = if cfg!(target_os = "windows") {
                "plause_probe.dll"
            } else if cfg!(target_os = "macos") {
                "libplause_probe.dylib"
            } else {
                "libplause_probe.so"
            };

            let path = target_dir.join("debug").join(file_name);
            assert!(
                path.exists(),
                "probe cdylib not found at {}",
                path.display()
            );
            path
        })
        .clone()
}

/// A scratch directory that cleans up after itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(name: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("plause-inspect-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        TempDir(dir)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn inspects_the_probe_completely() {
    let info = inspect(&probe_path()).expect("inspecting the probe should succeed");

    assert_eq!(info.plugins.len(), 1);
    let plugin = &info.plugins[0];

    // Descriptor
    let d = &plugin.descriptor;
    assert_eq!(d.id, "org.plause.probe");
    assert_eq!(d.name.as_deref(), Some("Plause Probe"));
    assert_eq!(d.vendor.as_deref(), Some("plause"));
    assert_eq!(d.version.as_deref(), Some("0.1.0"));
    assert!(d.features.iter().any(|f| f == "instrument"));
    assert!(d.features.iter().any(|f| f == "stereo"));

    // Extensions
    for ext in ["clap.audio-ports", "clap.note-ports", "clap.params"] {
        assert!(plugin.extensions.iter().any(|e| e == ext), "missing {ext}");
    }
    assert!(!plugin.extensions.iter().any(|e| e == "clap.gui"));

    // Audio ports: instrument — no inputs, one stereo main output.
    assert!(plugin.audio_ports.inputs.is_empty());
    assert_eq!(plugin.audio_ports.outputs.len(), 1);
    let out = &plugin.audio_ports.outputs[0];
    assert_eq!(out.id, 1);
    assert_eq!(out.name, "main");
    assert_eq!(out.channel_count, 2);
    assert!(out.is_main);
    assert_eq!(out.port_type.as_deref(), Some("stereo"));

    // Note ports: one each way, CLAP dialect.
    assert_eq!(plugin.note_ports.inputs.len(), 1);
    assert_eq!(plugin.note_ports.outputs.len(), 1);
    assert_eq!(plugin.note_ports.inputs[0].name, "notes in");
    assert_eq!(plugin.note_ports.outputs[0].name, "notes out");
    assert_eq!(plugin.note_ports.inputs[0].supported_dialects, vec!["clap"]);
    assert_eq!(
        plugin.note_ports.inputs[0].preferred_dialect.as_deref(),
        Some("clap")
    );

    // Params: continuous gain + stepped mode.
    assert_eq!(plugin.params.len(), 2);
    let gain = &plugin.params[0];
    assert_eq!((gain.id, gain.name.as_str()), (1, "Gain"));
    assert_eq!(
        (gain.min_value, gain.max_value, gain.default_value),
        (0.0, 1.0, 0.5)
    );
    assert!(gain.flags.iter().any(|f| f == "automatable"));
    assert!(!gain.flags.iter().any(|f| f == "stepped"));

    let mode = &plugin.params[1];
    assert_eq!((mode.id, mode.name.as_str()), (2, "Mode"));
    assert_eq!(
        (mode.min_value, mode.max_value, mode.default_value),
        (0.0, 3.0, 0.0)
    );
    assert!(mode.flags.iter().any(|f| f == "stepped"));
}

#[test]
fn json_output_is_stable_shape() {
    let info = inspect(&probe_path()).unwrap();
    let json = serde_json::to_value(&info).unwrap();

    // Spot-check the public JSON contract (field names are load-bearing:
    // plugin CI pipelines parse this).
    assert!(json["path"].is_string());
    let plugin = &json["plugins"][0];
    assert_eq!(plugin["descriptor"]["id"], "org.plause.probe");
    assert!(plugin["audio_ports"]["outputs"][0]["channel_count"].is_u64());
    assert!(plugin["params"][0]["default_value"].is_f64());
    assert!(plugin["note_ports"]["inputs"][0]["supported_dialects"].is_array());
}

#[test]
fn missing_path_is_a_distinct_error() {
    let err = inspect(&PathBuf::from("/definitely/not/here.clap")).unwrap_err();
    assert!(
        matches!(err, InspectError::Load(LoadError::NotFound { .. })),
        "expected NotFound, got: {err}"
    );
}

#[test]
fn a_file_that_is_not_a_dylib_reports_the_library_error() {
    let tmp = TempDir::new("junk");
    let junk = tmp.0.join("junk.clap");
    std::fs::write(&junk, b"this is not a dynamic library").unwrap();

    let err = inspect(&junk).unwrap_err();
    assert!(
        matches!(err, InspectError::Load(LoadError::LibraryLoad { .. })),
        "expected LibraryLoad, got: {err}"
    );
}

#[test]
fn a_dylib_without_clap_entry_reports_the_library_error() {
    // Compile a real (empty) cdylib with rustc so the failure is specifically
    // the missing `clap_entry` symbol, not an unloadable file.
    let tmp = TempDir::new("noentry");
    let src = tmp.0.join("empty.rs");
    std::fs::write(
        &src,
        "#[unsafe(no_mangle)]\npub extern \"C\" fn not_clap_entry() {}\n",
    )
    .unwrap();
    let out = tmp.0.join("noentry.clap");

    let rustc = std::env::var_os("RUSTC").unwrap_or_else(|| "rustc".into());
    let status = Command::new(rustc)
        .arg("--crate-type=cdylib")
        .arg(&src)
        .arg("-o")
        .arg(&out)
        .status()
        .expect("failed to invoke rustc");
    assert!(status.success(), "compiling the no-entry dylib failed");

    let err = inspect(&out).unwrap_err();
    assert!(
        matches!(err, InspectError::Load(LoadError::LibraryLoad { .. })),
        "expected LibraryLoad (missing clap_entry), got: {err}"
    );
    // The diagnostic should point the user at the actual problem.
    assert!(
        err.to_string().contains("clap_entry"),
        "unhelpful message: {err}"
    );
}
