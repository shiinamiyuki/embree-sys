use std::io::Result;
use std::process::Command;
use std::env;
fn download() -> Result<()> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = out_dir.clone() + &"/embree";
    if std::path::Path::new(&out_dir).exists() {
        return Ok(());
    }
    let (url, file) = if cfg!(target_os = "windows") {
        ("https://github.com/embree/embree/releases/download/v3.13.3/embree-3.13.3.x64.vc14.windows.zip",
        "embree.zip")
    } else if cfg!(target_os = "macos") {
        ("https://github.com/embree/embree/releases/download/v3.13.3/embree-3.13.3.x86_64.macosx.zip",
        "embree.zip")

    } else if cfg!(target_os = "linux") {
        ("https://github.com/embree/embree/releases/download/v3.13.3/embree-3.13.3.x86_64.linux.tar.gz",
        "embree.zip")
    } else {
        panic!("Not support platform")
    };
    Command::new("curl")
        .args(["-L", url, "--output", file])
        .output()
        .unwrap();
    dbg!(&out_dir);
    std::fs::create_dir_all(&out_dir).unwrap();
    Command::new("tar")
        .args(["-zxvf", file, "-C", &out_dir, "--strip-components=1"])
        .output()
        .unwrap();
    Ok(())
}
// fn gen() -> Result<()> {
//     let bindings = bindgen::Builder::default()
//         .header("./embree/include/embree3/rtcore.h")
//         .clang_arg("-I./embree/include")
//         .clang_arg("-I./embree/include/embree3")
//         .parse_callbacks(Box::new(bindgen::CargoCallbacks))
//         .generate()
//         .expect("Unable to generate bindings");
//     bindings
//         .write_to_file("src/binding.rs")
//         .expect("Couldn't write bindings!");
//     Ok(())
// }

// fn get_output_path() -> PathBuf {
//     //<root or manifest path>/target/<profile>/
//     let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
//     let build_type = env::var("PROFILE").unwrap();
//     let path = Path::new(&manifest_dir_string)
//         .join("target")
//         .join(build_type);
//     return PathBuf::from(path);
// }

fn main() -> Result<()> {
    download().unwrap();
    // gen()?;
    println!("{:?}", env::var("OUT_DIR"));
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/embree/bin/", out_dir);
    println!("cargo:rustc-link-search=native={}/embree/lib/", out_dir);
    println!("cargo:rustc-link-lib=dylib=embree3");
    #[cfg(target_os = "windows")]
    let out_dir = out_dir.clone() + &"/embree/bin";
    #[cfg(target_os = "windows")]
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some() && path.extension().unwrap() == "dll" {
            // let target_dir = get_output_path();
            let comps: Vec<_> = path.components().collect();
            let dest =
                std::path::PathBuf::from_iter(comps[..comps.len() - 6].iter()).join(path.file_name().unwrap());
            println!("{:?}", path);
            println!("{:?}", dest);
            std::fs::copy(path, dest).unwrap();
        }
    }
    Ok(())
}
