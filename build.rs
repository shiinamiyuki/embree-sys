use std::env;
use std::io::Result;
use std::process::Command;
fn download() -> Result<String> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_dir = out_dir.clone() + &"/embree";
    if std::path::Path::new(&out_dir).exists() {
        return Ok(out_dir);
    }
    let (url, file) = (
        "https://github.com/embree/embree/archive/refs/tags/v3.13.4.zip",
        "embree.zip",
    );
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
    Ok(out_dir)
}
fn gen(out_dir:&String) -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/embree3/rtcore.h", out_dir))
        .clang_arg("-I./embree/include")
        .clang_arg("-I./embree/include/embree3")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false)
        .generate()
        .expect("Unable to generate bindings");
    bindings
        .write_to_file("src/binding.rs")
        .expect("Couldn't write bindings!");
    Ok(())
}

// fn get_output_path() -> PathBuf {
//     //<root or manifest path>/target/<profile>/
//     let manifest_dir_string = env::var("CARGO_MANIFEST_DIR").unwrap();
//     let build_type = env::var("PROFILE").unwrap();
//     let path = Path::new(&manifest_dir_string)
//         .join("target")
//         .join(build_type);
//     return PathBuf::from(path);
// }

fn compile(out_dir:&String)->Result<()> {
    cmake::Config::new(out_dir)
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBREE_ISPC_SUPPORT", "OFF")
        .define("EMBREE_TASKING_SYSTEM", "INTERNAL")
        .build();
    Ok(())
}
fn main() -> Result<()> {
    let out_dir = download().unwrap();
    compile(&out_dir)?;
    gen(&out_dir)?;
//     println!("{:?}", env::var("OUT_DIR"));
//     let out_dir = env::var("OUT_DIR").unwrap();
//     println!("cargo:rustc-link-search=native={}/embree/bin/", out_dir);
//     println!("cargo:rustc-link-search=native={}/embree/lib/", out_dir);
//     println!("cargo:rustc-link-lib=dylib=embree3");

//     let out_dir = if cfg!(target_os = "windows") {
//         out_dir.clone() + &"/embree/bin"
//     } else {
//         out_dir.clone() + &"/embree/lib"
//     };

//     for entry in std::fs::read_dir(out_dir)? {
//         let entry = entry?;
//         let path = entry.path();
//         if path.extension().is_some()
//             && (path.extension().unwrap() == "dll" || path.extension().unwrap() == "so")
//         {
//             // let target_dir = get_output_path();
//             let comps: Vec<_> = path.components().collect();
//             let dest = std::path::PathBuf::from_iter(comps[..comps.len() - 6].iter())
//                 .join(path.file_name().unwrap());
//             println!("{:?}", path);
//             println!("{:?}", dest);
//             std::fs::copy(path, dest).unwrap();
//         }
//     }
    Ok(())
}
