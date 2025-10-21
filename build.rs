use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let sysdic_src = find_sysdic(&manifest_dir)
        .unwrap_or_else(|| panic!("sysdic directory not found under manifest dir: {}", manifest_dir));
    let sysdic_dst = Path::new(&out_dir).join("sysdic");

    // Create destination directory
    if let Err(e) = fs::create_dir_all(&sysdic_dst) {
        panic!("Failed to create sysdic directory in OUT_DIR: {}", e);
    }

    // Copy all sysdic files to OUT_DIR
    copy_dir_contents(&sysdic_src, &sysdic_dst).unwrap_or_else(|e| {
        panic!("Failed to copy sysdic files from {:?}: {}", sysdic_src, e);
    });

    // Tell Cargo to rerun this build script if sysdic changes
    let manifest_path = Path::new(&manifest_dir);
    let rerun_path = sysdic_src
        .strip_prefix(manifest_path)
        .unwrap_or(&sysdic_src);
    println!("cargo:rerun-if-changed={}", rerun_path.display());

    // Set environment variable for runtime path lookup
    println!("cargo:rustc-env=SYSDIC_PATH={}", sysdic_dst.display());
}

fn find_sysdic(manifest_dir: &str) -> Option<PathBuf> {
    let candidates = [
        Path::new(manifest_dir).join("sysdic"),
        Path::new(manifest_dir).join("runome").join("sysdic"),
    ];

    candidates.into_iter().find(|path| path.exists())
}

fn copy_dir_contents(src: &Path, dst: &Path) -> std::io::Result<()> {
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            fs::create_dir_all(&dst_path)?;
            copy_dir_contents(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
