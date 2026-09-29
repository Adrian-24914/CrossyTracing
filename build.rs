use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

const MAPS: [&str; 2] = ["basecolor", "normal"];

fn main() {
    let project = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let textures_root = project.join("assets/textures");
    let converter = project.join("tools/convert_texture.ps1");

    println!("cargo:rerun-if-changed={}", converter.display());
    let mut materials = fs::read_dir(&textures_root)
        .unwrap_or_else(|error| panic!("no se pudo leer {}: {error}", textures_root.display()))
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    materials.sort();

    for material in materials {
        for map in MAPS {
            let Some(source) = find_source(&material, map) else {
                continue;
            };
            println!("cargo:rerun-if-changed={}", source.display());

            let destination = material.join("runtime").join(format!("{map}.bmp"));
            if is_current(&source, &destination) {
                continue;
            }

            fs::create_dir_all(destination.parent().unwrap()).unwrap_or_else(|error| {
                panic!(
                    "no se pudo crear {}: {error}",
                    destination.parent().unwrap().display()
                )
            });
            convert(&converter, &source, &destination);
        }
    }
}

fn find_source(directory: &Path, map: &str) -> Option<PathBuf> {
    let mut candidates = fs::read_dir(directory)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .map(|entry| entry.path())
        .filter(|path| {
            matches!(
                path.extension()
                    .and_then(|extension| extension.to_str())
                    .map(|extension| extension.to_ascii_lowercase())
                    .as_deref(),
                Some("jpg" | "jpeg" | "png" | "bmp")
            )
        })
        .filter(|path| {
            let stem = path
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            stem == map || stem.ends_with(&format!("_{map}"))
        })
        .collect::<Vec<_>>();
    candidates.sort();
    candidates.into_iter().next()
}

fn is_current(source: &Path, destination: &Path) -> bool {
    let Ok(source_time) = fs::metadata(source).and_then(|metadata| metadata.modified()) else {
        return false;
    };
    let Ok(destination_time) = fs::metadata(destination).and_then(|metadata| metadata.modified())
    else {
        return false;
    };
    destination_time >= source_time
}

fn convert(converter: &Path, source: &Path, destination: &Path) {
    let mut launch_error = None;
    for shell in ["powershell.exe", "pwsh.exe"] {
        match Command::new(shell)
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(converter)
            .arg("-Source")
            .arg(source)
            .arg("-Destination")
            .arg(destination)
            .arg("-Size")
            .arg("256")
            .status()
        {
            Ok(status) if status.success() => return,
            Ok(status) => panic!(
                "falló la conversión de {} a {} ({status})",
                source.display(),
                destination.display()
            ),
            Err(error) => launch_error = Some(error),
        }
    }

    panic!(
        "no se pudo ejecutar PowerShell para convertir {}: {}",
        source.display(),
        launch_error.unwrap()
    );
}
