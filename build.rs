use std::io::{copy, Result};
use std::path::PathBuf;
use std::process::Command;
fn download() -> reqwest::Result<()> {
    if std::path::Path::new("./embree").exists() {
        return Ok(());
    }
    let (url, file) = if cfg!(target_os = "windows") {
        ("https://github.com/embree/embree/releases/download/v3.13.2/embree-3.13.2.x64.vc14.windows.zip",
        "embree.zip")
    } else {
        todo!()
    };
    let response = reqwest::blocking::get(url)?;
    let mut dst = std::fs::File::create(file).unwrap();
    let mut content = std::io::Cursor::new(response.bytes()?);
    copy(&mut content, &mut dst).unwrap();
    std::fs::create_dir_all("./embree").unwrap();
    Command::new("tar")
        .args(["-zxvf", file, "-C", "./embree", "--strip-components=1"])
        .output()
        .unwrap();

    Ok(())
}
fn gen() -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header("./embree/include/embree3/rtcore.h")
        .clang_arg("-I./embree/include")
        .clang_arg("-I./embree/include/embree3")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file("src/binding.rs")
        .expect("Couldn't write bindings!");
    Ok(())
}

fn main() -> Result<()> {
    download().unwrap();
    // gen()?;
    println!("cargo:rustc-link-search=native=./embree/bin");
    println!("cargo:rustc-link-search=native=./embree/lib");
    println!("cargo:rustc-link-lib=dylib=embree3");
    Ok(())
}
