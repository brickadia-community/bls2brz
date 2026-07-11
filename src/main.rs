use bls2brz::{bls, convert};
use std::{
    ffi::OsStr,
    path::{Path, PathBuf},
};

fn main() {
    eprintln!("{} v{}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
    eprintln!();

    if let Err(e) = run() {
        eprintln!("{}", e);
        eprintln!();
        wexit::prompt_enter_to_exit(1);
    }

    eprintln!();
    wexit::prompt_enter_to_exit(0);
}

fn run() -> Result<(), String> {
    let args = parse_args()
        .map_err(|_| String::from("Error: No bls files given. Drag them onto this program's executable file. (Not this window! This is just an error message, not the program itself.)"))?;

    for (i, input_path) in args.input_paths.iter().enumerate() {
        if i > 0 {
            println!();
        }

        let input_path = PathBuf::from(input_path);

        println!("Converting {}", input_path.display());

        if input_path.extension() != Some(OsStr::new("bls")) {
            println!("Extension is not .bls, skipping");
            continue;
        }

        let mut output_path = input_path.clone();

        output_path.set_extension("brz");

        convert_one(&input_path, &output_path)
            .map_err(|e| format!("Error converting {}: {}", input_path.display(), e))?;
    }

    Ok(())
}

fn convert_one(input_path: impl AsRef<Path>, output_path: impl AsRef<Path>) -> Result<(), String> {
    let input_path = input_path.as_ref();
    let output_path = output_path.as_ref();

    let save = errmsg(bls::Save::from_path(input_path), "Failed to read bls file")?;

    let mut converted = convert(&save);
    let converted_brick_count = converted.world.bricks.len()
        + converted
            .world
            .grids
            .iter()
            .map(|(_, bricks)| bricks.len())
            .sum::<usize>();

    // A Blockland save screenshot sits next to the .bls with the same name and a
    // .jpg extension. When present, use it as the prefab preview
    // (Meta/Screenshot.jpg) — the game reads it for the prefab thumbnail.
    let screenshot_path = input_path.with_extension("jpg");
    if screenshot_path.is_file() {
        match std::fs::read(&screenshot_path) {
            Ok(bytes) => {
                println!("Using preview {}", screenshot_path.display());
                converted.world.meta.screenshot = Some(bytes);
            }
            Err(e) => println!(
                "Failed to read preview {}: {}",
                screenshot_path.display(),
                e
            ),
        }
    }

    if let Some(file_name) = input_path.file_name() {
        let description = &mut converted.world.meta.bundle.description;

        let mut prefix = format!(
            "Converted from {} with bls2brz.\nBrick count: {} Blockland → {} Brickadia.",
            file_name.to_string_lossy(),
            converted.count_success,
            converted_brick_count,
        );

        if !description.is_empty() {
            prefix.push('\n');
        }

        description.insert_str(0, &prefix);
    }

    if !converted.unknown_ui_names.is_empty() {
        println!("Unknown bricks:");
        let mut ui_names: Vec<_> = converted.unknown_ui_names.into_iter().collect();
        ui_names.sort_by(|(_, ac), (_, bc)| ac.cmp(bc).reverse());
        for (ui_name, count) in ui_names {
            let ui_name = if ui_name != ui_name.trim() {
                format!("{:?}", ui_name)
            } else {
                ui_name
            };
            println!("  {:<28} {:>4} bricks", ui_name, count);
        }
    }

    if !converted.unconverted_lights.is_empty() {
        println!("Unsupported lights:");
        let mut names: Vec<_> = converted.unconverted_lights.into_iter().collect();
        names.sort_by(|(_, ac), (_, bc)| ac.cmp(bc).reverse());
        for (name, count) in names {
            println!("  {:<28} {:>4} lights", name, count);
        }
    }

    if converted.count_failure > 0 {
        println!("{} bricks failed to convert", converted.count_failure);
    }

    println!(
        "{} of {} bricks converted successfully to {} bricks",
        converted.count_success,
        converted.count_success + converted.count_failure,
        converted_brick_count,
    );

    // Brickadia places these outputs with the prefab tool, which needs
    // `Meta/Prefab.json` (bundle type "Prefab" plus pivot centroid/bounds
    // computed from the brick bounding box). Without it the game logs
    // "Failed to find prefab meta, placement will act oddly."
    converted.world.make_prefab();

    // Dispatch on the output extension: .brdb writes the sqlite directory format,
    // anything else (default .brz) writes the compressed archive.
    let is_brdb = output_path.extension() == Some(OsStr::new("brdb"));
    let result = if is_brdb {
        converted.world.write_brdb(output_path)
    } else {
        converted.world.write_brz(output_path)
    };

    errmsg(result, "Failed to write save file")?;

    Ok(())
}

struct Args {
    input_paths: Vec<String>,
}

fn parse_args() -> Result<Args, ()> {
    let mut args = std::env::args();
    args.next().unwrap();

    let input_paths: Vec<_> = args.collect();

    if input_paths.is_empty() {
        return Err(());
    }

    Ok(Args { input_paths })
}

fn errmsg<T, E: std::fmt::Display>(r: Result<T, E>, message_prefix: &str) -> Result<T, String> {
    r.map_err(|e| format!("{}: {}", message_prefix, e))
}
