use std::env;
use std::fs;
use std::path::Path;

fn generate_help_pages() -> std::io::Result<()> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let pages_dir = Path::new(manifest_dir).join("help");
    let out_dir = env::var("OUT_DIR").expect("OUT_DIR not set");
    let dest = Path::new(&out_dir).join("help_pages.rs");

    println!("cargo::rerun-if-changed={}", pages_dir.display());

    let mut stems: Vec<String> = fs::read_dir(&pages_dir)
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("md") {
                return None;
            }
            println!("cargo::rerun-if-changed={}", path.display());
            path.file_stem().and_then(|stem| stem.to_str()).map(str::to_string)
        })
        .collect();

    stems.sort();

    let mut generated = String::from("pub static HELP_PAGES: &[(&str, &str)] = &[\n");
    for stem in &stems {
        let path = pages_dir.join(format!("{stem}.md"));
        generated.push_str(&format!("    ({stem:?}, include_str!({:?})),\n", path.display().to_string()));
    }
    generated.push_str("];\n");

    fs::write(dest, generated)
}

fn main() -> std::io::Result<()> {
    generate_help_pages()?;

    #[cfg(windows)]
    {
        println!("cargo::rerun-if-changed=../kore/assets/icon.ico");

        let mut res = winres::WindowsResource::new();
        res.set_icon("../kore/assets/icon.ico");
        res.compile()?;
    }

    Ok(())
}
