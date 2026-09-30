use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let project = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let ui_root = project.join("assets/ui");
    let converter = project.join("tools/convert_ui_image.ps1");
    println!("cargo:rerun-if-changed={}", ui_root.display());
    println!("cargo:rerun-if-changed={}", converter.display());

    for (name, width, height) in [("intro", 480, 300), ("death", 600, 120)] {
        let Some(source) = find_source(&ui_root, name) else {
            continue;
        };
        println!("cargo:rerun-if-changed={}", source.display());
        let destination = ui_root.join("runtime").join(format!("{name}.bmp"));
        if is_current(&source, &destination) {
            continue;
        }
        fs::create_dir_all(destination.parent().unwrap()).unwrap();
        convert(&converter, &source, &destination, width, height);
    }
}

fn find_source(root: &Path, name: &str) -> Option<PathBuf> {
    ["png", "jpg", "jpeg", "bmp"]
        .into_iter()
        .map(|extension| root.join(format!("{name}.{extension}")))
        .find(|path| path.is_file())
}

fn is_current(source: &Path, destination: &Path) -> bool {
    let Ok(source_time) = fs::metadata(source).and_then(|metadata| metadata.modified()) else {
        return false;
    };
    fs::metadata(destination)
        .and_then(|metadata| metadata.modified())
        .is_ok_and(|destination_time| destination_time >= source_time)
}

fn convert(converter: &Path, source: &Path, destination: &Path, width: u32, height: u32) {
    for shell in ["powershell.exe", "pwsh.exe"] {
        let Ok(status) = Command::new(shell)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(converter)
            .arg("-Source")
            .arg(source)
            .arg("-Destination")
            .arg(destination)
            .arg("-MaximumWidth")
            .arg(width.to_string())
            .arg("-MaximumHeight")
            .arg(height.to_string())
            .status()
        else {
            continue;
        };
        if status.success() {
            return;
        }
        panic!(
            "falló la conversión de {} a {}",
            source.display(),
            destination.display()
        );
    }
    panic!(
        "no se pudo iniciar PowerShell para convertir {}",
        source.display()
    );
}
