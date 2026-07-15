//! Finding `.clap` bundles on disk.
//!
//! On macOS a `.clap` is a bundle *directory*; on Linux and Windows it is a
//! single file. [`scan`] treats any entry with a `.clap` extension as a plugin
//! and does not descend into bundle directories.

use std::io;
use std::path::{Path, PathBuf};

/// The standard CLAP search paths for this platform, per the CLAP spec's
/// `entry.h`, preceded by any paths in the `CLAP_PATH` environment variable.
///
/// Paths are returned whether or not they exist; [`scan`] treats a missing
/// directory as empty.
pub fn default_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Some(clap_path) = std::env::var_os("CLAP_PATH") {
        paths.extend(std::env::split_paths(&clap_path));
    }

    #[cfg(target_os = "macos")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join("Library/Audio/Plug-Ins/CLAP"));
        }
        paths.push(PathBuf::from("/Library/Audio/Plug-Ins/CLAP"));
    }

    #[cfg(target_os = "linux")]
    {
        if let Some(home) = std::env::var_os("HOME") {
            paths.push(PathBuf::from(home).join(".clap"));
        }
        paths.push(PathBuf::from("/usr/lib/clap"));
    }

    #[cfg(target_os = "windows")]
    {
        if let Some(common) = std::env::var_os("COMMONPROGRAMFILES") {
            paths.push(PathBuf::from(common).join("CLAP"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(local).join("Programs/Common/CLAP"));
        }
    }

    paths
}

/// Recursively collect every `.clap` bundle under `dir`, sorted by path.
///
/// A missing or non-directory `dir` yields an empty list rather than an error,
/// so callers can scan all of [`default_search_paths`] unconditionally.
pub fn scan(dir: &Path) -> io::Result<Vec<PathBuf>> {
    let mut found = Vec::new();
    if dir.is_dir() {
        walk(dir, &mut found)?;
    }
    found.sort();
    Ok(found)
}

fn walk(dir: &Path, found: &mut Vec<PathBuf>) -> io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path
            .extension()
            .is_some_and(|ext| ext.eq_ignore_ascii_case("clap"))
        {
            found.push(path);
        } else if path.is_dir() {
            walk(&path, found)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    struct TempTree(PathBuf);

    impl TempTree {
        fn new(name: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("plause-discovery-{name}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&root);
            fs::create_dir_all(&root).unwrap();
            TempTree(root)
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn finds_files_and_bundle_dirs_but_does_not_descend_into_bundles() {
        let tree = TempTree::new("scan");
        let root = &tree.0;

        // A .clap file (Linux/Windows style), nested one level down.
        fs::create_dir(root.join("vendor")).unwrap();
        fs::write(root.join("vendor/synth.clap"), b"").unwrap();
        // A .clap bundle directory (macOS style) containing a decoy.
        fs::create_dir_all(root.join("Tonarch.clap/Contents")).unwrap();
        fs::write(root.join("Tonarch.clap/Contents/inner.clap"), b"").unwrap();
        // Noise that must be ignored.
        fs::write(root.join("readme.txt"), b"").unwrap();

        let found = scan(root).unwrap();
        assert_eq!(
            found,
            vec![root.join("Tonarch.clap"), root.join("vendor/synth.clap")]
        );
    }

    #[test]
    fn missing_directory_is_empty_not_an_error() {
        let tree = TempTree::new("missing");
        assert_eq!(
            scan(&tree.0.join("does-not-exist")).unwrap(),
            Vec::<PathBuf>::new()
        );
    }
}
