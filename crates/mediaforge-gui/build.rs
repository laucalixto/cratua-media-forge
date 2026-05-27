// build.rs — Copy bundled ffmpeg to target directory after native build.
// Skipped during cross-compilation.
use std::path::Path;

fn main() {
    // Skip during cross-compilation (TARGET != HOST)
    let target = std::env::var("TARGET").unwrap_or_default();
    let host = std::env::var("HOST").unwrap_or_default();
    if target != host {
        return;
    }

    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap_or(manifest_dir);

    #[cfg(target_os = "windows")]
    let ffmpeg_name = "ffmpeg.exe";
    #[cfg(not(target_os = "windows"))]
    let ffmpeg_name = "ffmpeg";

    let ffmpeg_src = project_root.join("vendor").join("ffmpeg").join(ffmpeg_name);

    if ffmpeg_src.exists() {
        let out_dir_str = std::env::var("OUT_DIR").unwrap();
        let out_dir = Path::new(&out_dir_str);
        let target_dir = out_dir
            .parent()
            .and_then(|p| p.parent())
            .and_then(|p| p.parent());

        if let Some(target_dir) = target_dir {
            let dest = target_dir.join(ffmpeg_name);
            if !dest.exists() {
                println!("cargo:warning=Copying bundled ffmpeg to {}", dest.display());
                let _ = std::fs::copy(&ffmpeg_src, &dest);
            }
        }
    }
}
