// build.rs — Windows resource compilation + ffmpeg bundling for native builds.
use std::path::Path;
use std::process::Command;

fn main() {
    let target = std::env::var("TARGET").unwrap_or_default();
    let host = std::env::var("HOST").unwrap_or_default();
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let project_root = manifest_dir.parent().unwrap_or(manifest_dir);

    // ── Windows resource (.rc → .o) for .exe icon and version info ──
    if target.contains("windows") || target.contains("mingw") {
        let rc_path = manifest_dir.join("resource.rc");
        if rc_path.exists() {
            let out_dir = std::env::var("OUT_DIR").unwrap();
            let obj_path = Path::new(&out_dir).join("resource.o");

            let windres = if target.contains("windows") && host.contains("linux") {
                "x86_64-w64-mingw32-windres"
            } else {
                "windres"
            };

            let status = Command::new(windres)
                .current_dir(&manifest_dir)
                .arg(&rc_path)
                .arg(&obj_path)
                .status();

            match status {
                Ok(s) if s.success() => {
                    println!("cargo:rustc-link-arg={}", obj_path.display());
                }
                Ok(_) => {
                    println!("cargo:warning=windres failed — .exe will have no icon");
                }
                Err(_) => {
                    println!("cargo:warning=windres not found — .exe will have no icon");
                }
            }
        }
    }

    // ── Copy bundled ffmpeg (native builds only, not cross-compilation) ──
    if target != host {
        return;
    }

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
