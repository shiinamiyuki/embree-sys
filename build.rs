use std::io::{copy, Result};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
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

fn get_output_path() -> PathBuf {
    //<root or manifest path>/target/<profile>/
    let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
    let build_type = env::var("PROFILE").unwrap();
    let path = Path::new(&manifest_dir_string)
        .join("target")
        .join(build_type);
    return PathBuf::from(path);
}

fn main() -> Result<()> {
    download().unwrap();
    // gen()?;
    println!("{:?}", env::var("OUT_DIR"));
    println!("cargo:rustc-link-search=native=./embree/bin/");
    println!("cargo:rustc-link-search=native=./embree/lib/");
    println!("cargo:rustc-link-lib=dylib=embree3");

    #[cfg(target_os = "windows")]
    for entry in fs::read_dir("./embree/bin/")? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some() && path.extension().unwrap() == "dll" {
            let target_dir = get_output_path();
            let dest = Path::join(Path::new(&target_dir), path.file_name().unwrap());
            fs::copy(path, dest).unwrap();
        }
    }
    Ok(())
}
