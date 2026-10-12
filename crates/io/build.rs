//! Content fingerprint: cached pictures must not survive geometry/placement changes.
use sha2::{Digest, Sha256};
use std::{io, path::Path};

fn sources(path: &Path, files: &mut Vec<std::path::PathBuf>) -> io::Result<()> {
    println!("cargo:rerun-if-changed={}", path.display());
    for entry in std::fs::read_dir(path)? {
        let path = entry?.path();
        if path.is_dir() {
            sources(&path, files)?;
        } else if path.extension().is_some_and(|e| e == "rs") {
            files.push(path);
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    let root =
        std::path::PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").ok_or_else(|| io::Error::other("missing manifest directory"))?).join("../..");
    let mut files = Vec::new();
    for name in ["geom", "sketch", "kernel", "doc", "io", "engine", "render"] {
        sources(&root.join("crates").join(name).join("src"), &mut files)?;
    }
    for name in ["truck-meshalgo", "truck-stepio", "truck-shapeops", "matext4cgmath"] {
        sources(&root.join("vendor").join(name).join("src"), &mut files)?;
    }
    files.push(root.join("Cargo.lock"));
    files.push(root.join("Cargo.toml"));
    files.push(root.join("crates/io/build.rs"));
    files.sort();
    let mut hash = Sha256::new();
    hash.update(b"solvecraft-provisional-display/1;millimetres;Z-up");
    for path in files {
        println!("cargo:rerun-if-changed={}", path.display());
        hash.update(path.strip_prefix(&root).map_err(io::Error::other)?.to_string_lossy().replace('\\', "/").as_bytes());
        let bytes = std::fs::read(path)?;
        hash.update((bytes.len() as u64).to_le_bytes());
        hash.update(bytes);
    }
    println!("cargo:rustc-env=SOLVECRAFT_DISPLAY_BUILD={:x}", hash.finalize());
    Ok(())
}
