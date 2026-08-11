#[cfg(windows)]
fn main() -> std::io::Result<()> {
    println!("cargo::rerun-if-changed=../core/assets/icon.ico");

    let mut res = winres::WindowsResource::new();
    res.set_icon("../core/assets/icon.ico");
    res.compile()
}

#[cfg(not(windows))]
fn main() -> std::io::Result<()> {
    Ok(())
}
