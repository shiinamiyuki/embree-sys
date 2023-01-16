use std::env;
use std::io::Result;
use std::process::Command;

fn build_embree() -> Result<String> {
    let out_dir = env::var("OUT_DIR").unwrap();

    cmake::Config::new("embree")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBREE_ISPC_SUPPORT", "OFF")
        .define("EMBREE_TASKING_SYSTEM", "INTERNAL")
        .define("EMBREE_TUTORIALS", "OFF")
        .define("EMBREE_STATIC_LIB", "OFF")
        .generator("Ninja")
        .build();

    Ok(out_dir)
}
fn gen(out_dir: &String) -> Result<()> {
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

fn main() -> Result<()> {
    let out_dir = build_embree().unwrap();
    gen(&out_dir)?;
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/bin/", out_dir);
    println!("cargo:rustc-link-search=native={}/lib/", out_dir);
    println!("cargo:rustc-link-lib=dylib=embree3");

    let out_dir = if cfg!(target_os = "windows") {
        out_dir.clone() + &"/bin"
    } else {
        out_dir.clone() + &"/lib"
    };
    let mut count = 0;
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some()
            && (path.extension().unwrap() == "dll" ||
                path.extension().unwrap() == "so" ||
                path.extension().unwrap() == "dylib")
        {
            let comps: Vec<_> = path.components().collect();
            let dest = std::path::PathBuf::from_iter(comps[..comps.len() - 5].iter())
                .join(path.file_name().unwrap());
            println!("{:?}", path);
            println!("{:?}", dest);
            std::fs::copy(path, dest).unwrap();
            count += 1;
        }
    }
    assert!(count > 0, "No dlls or so files found");
    Ok(())
}
