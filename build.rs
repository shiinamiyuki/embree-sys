use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn build_embree() -> Result<String> {
    let out_dir = env::var("OUT_DIR").unwrap();

    cmake::Config::new("embree")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBREE_ISPC_SUPPORT", "OFF")
        .define("EMBREE_TASKING_SYSTEM", "INTERNAL")
        .define("EMBREE_TUTORIALS", "OFF")
        .define("EMBREE_STATIC_LIB", "OFF")
        .define("EMBREE_GEOMETRY_QUAD", "OFF")
        .define("EMBREE_GEOMETRY_CURVE", "OFF")
        .define("EMBREE_GEOMETRY_SUBDIVISION", "OFF")
        .define("EMBREE_GEOMETRY_POINT", "OFF")
        .generator("Ninja")
        .build();

    Ok(out_dir)
}
fn gen(out_dir: &String) -> Result<()> {
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/embree4/rtcore.h", out_dir))
        .clang_arg("-I./embree/include")
        .clang_arg("-I./embree/include/embree4")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false)
        .generate()
        .unwrap();
    bindings
        .write_to_file("src/binding.rs")
        .expect("Couldn't write bindings!");
    Ok(())
}

fn copy_dlls(out_dir: &PathBuf) {
    let mut out_dir = out_dir.clone();

    dbg!(&out_dir);
    for entry in std::fs::read_dir(out_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.extension().is_some()
            && (path.extension().unwrap() == "dll"
                || path.extension().unwrap() == "so"
                || path.extension().unwrap() == "dylib")
        {
            // let target_dir = get_output_path();
            let comps: Vec<_> = path.components().collect();
            let copy_if_different = |src, dst| {
                let p_src = Path::new(&src);
                let p_dst = Path::new(&dst);
                let should_copy = p_dst.exists();
                let check_should_copy = || -> Option<bool> {
                    let src_metadata = fs::metadata(p_src).ok()?;
                    let dst_metadata = fs::metadata(p_dst).ok()?;
                    Some(src_metadata.modified().ok()? != dst_metadata.modified().ok()?)
                };
                let should_copy = should_copy || check_should_copy().unwrap_or(true);
                if should_copy {
                    std::fs::copy(p_src, p_dst).unwrap();
                }
            };
            {
                let dest = std::path::PathBuf::from_iter(comps[..comps.len() - 5].iter())
                    .join(path.file_name().unwrap());
                copy_if_different(&path, dest);
            }
            {
                let dest = std::path::PathBuf::from_iter(comps[..comps.len() - 5].iter())
                    .join("deps")
                    .join(path.file_name().unwrap());
                dbg!(&dest);
                copy_if_different(&path, dest);
            }
        }
    }
}

fn main() -> Result<()> {
    let out_dir = build_embree().unwrap();
    gen(&out_dir)?;
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/bin/", out_dir);
    println!("cargo:rustc-link-search=native={}/lib/", out_dir);
    println!("cargo:rustc-link-lib=dylib=embree4");

    let out_dir = if cfg!(target_os = "windows") {
        out_dir.clone() + &"/bin"
    } else {
        out_dir.clone() + &"/lib"
    };
    copy_dlls(&PathBuf::from(out_dir));
    Ok(())
}
