#[cfg(windows)]
fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-changed=../kore/assets/icon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon("../kore/assets/icon.ico");
    res.compile()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    Ok(())
}
