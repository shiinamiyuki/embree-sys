use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

fn build_embree() -> Result<String> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let generator = env::var("CMAKE_GENERATOR").unwrap_or("Ninja".to_string());

    cmake::Config::new("embree")
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBREE_ISPC_SUPPORT", "OFF")
        .define("EMBREE_TASKING_SYSTEM", "INTERNAL")
        .define("EMBREE_TUTORIALS", "OFF")
        .define("EMBREE_STATIC_LIB", "OFF")
        .define("EMBREE_GEOMETRY_QUAD", "OFF")
        .define("EMBREE_GEOMETRY_CURVE", "OFF")
        .define("EMBREE_GEOMETRY_SUBDIVISION", "OFF")
        .define("EMBREE_RAY_MASK", "ON")
        .define("EMBREE_GEOMETRY_POINT", "OFF")
        .define("CMAKE_MACOSX_RPATH", "ON")
        .define("CMAKE_SKIP_BUILD_RPATH", "OFF")
        .define("CMAKE_BUILD_RPATH_USE_ORIGIN", "ON")
        .define("CMAKE_BUILD_WITH_INSTALL_RPATH", "ON")
        .define("CMAKE_INSTALL_RPATH", if cfg!(target_os = "linux") {
            "$ORIGIN"
        } else if cfg!(target_os = "macos") {
            "@loader_path"
        } else {
            ""
        })
        .generator(generator)
        .build();

    Ok(out_dir)
}

fn gen(out_dir: &String) -> Result<()> {
    match env::var("GEN_BINDING") {
        Ok(_) => {}
        Err(_) => return Ok(()),
    }
    let bindings = bindgen::Builder::default()
        .header(format!("{}/include/embree4/rtcore.h", out_dir))
        .clang_arg("-I./embree/include")
        .clang_arg("-I./embree/include/embree4")
        .allowlist_function("rtc.*")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        .prepend_enum_name(false)
        .generate()
        .unwrap();
    bindings
        .write_to_file("src/binding.rs")
        .expect("Couldn't write bindings!");
    Ok(())
}

fn is_path_dll(path: &PathBuf) -> bool {
    let basic_check = path.extension().is_some()
        && (path.extension().unwrap() == "dll"
        || path.extension().unwrap() == "lib" // lib is also need on Windows for linking DLLs
        || path.extension().unwrap() == "so"
        || path.extension().unwrap() == "dylib");
    if basic_check {
        return true;
    }
    if cfg!(target_os = "linux") {
        if let Some(stem) = path.file_stem() {
            if let Some(ext) = PathBuf::from(stem).extension() {
                if ext == "so" {
                    return true;
                }
            }
        }
    }
    false
}

fn copy_dlls(src_dir: &PathBuf, dst_dir: &PathBuf) {
    let out_dir = src_dir.clone();
    for entry in std::fs::read_dir(out_dir).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if is_path_dll(&path) {
            let copy_if_different = |src, dst| {
                let p_src = Path::canonicalize(src).unwrap();
                let p_src = p_src.as_path();
                let p_dst = Path::new(&dst);
                let should_copy = !p_dst.exists();
                let check_should_copy = || -> Option<bool> {
                    let src_metadata = fs::metadata(p_src).ok()?;
                    let dst_metadata = fs::metadata(p_dst).ok()?;
                    Some(src_metadata.modified().ok()? != dst_metadata.modified().ok()?)
                };
                let should_copy = should_copy || check_should_copy().unwrap_or(true);
                dbg!(&p_src, &p_dst, should_copy);
                if should_copy {
                    std::fs::copy(p_src, p_dst).unwrap();
                }
            };
            {
                let dest = dst_dir.clone().join(path.file_name().unwrap());
                copy_if_different(&path, dest);
            }
            // {
            //     let dest = dst_dir.clone().join("deps").join(path.file_name().unwrap());
            //     copy_if_different(&path, dest);
            // }
            // {
            //     let dest = dst_dir
            //         .clone()
            //         .join("examples")
            //         .join(path.file_name().unwrap());
            //     copy_if_different(&path, dest);
            // }
        }
    }
}

fn prebuild_available() -> bool {
    let force_build = env::var("EMBREE_FORCE_BUILD_FROM_SOURCE").unwrap_or("0".to_string());
    let force_build = force_build.to_uppercase();
    let force_build = force_build == "1" || force_build == "ON" || force_build == "TRUE";
    !force_build && cfg!(target_arch = "x86_64") && (cfg!(target_os = "windows") || cfg!(target_os = "linux"))
}

fn download_embree() {
    let linux_url = r#"https://github.com/embree/embree/releases/download/v4.1.0/embree-4.1.0.x86_64.linux.tar.gz"#;
    let windows_url =
        r#"https://github.com/embree/embree/releases/download/v4.1.0/embree-4.1.0.x64.windows.zip"#;
    let source_url = r#"https://github.com/embree/embree/archive/refs/tags/v4.1.0.tar.gz"#;
    let out_dir = "embree";
    if prebuild_available() {
        let url = if cfg!(target_os = "windows") {
            windows_url
        } else {
            linux_url
        };
        let filename = if cfg!(target_os = "windows") {
            "embree.zip"
        } else {
            "embree.tar.gz"
        };
        Command::new("curl")
            .arg("-L")
            .arg(url)
            .arg("--output")
            .arg(filename)
            .output()
            .unwrap();
        std::fs::create_dir_all(&out_dir).unwrap();
        Command::new("tar")
            .args(["-zxvf", filename, "-C", &out_dir, "--strip-components=1"])
            .output()
            .unwrap();
    } else {
        Command::new("curl")
            .arg("-L")
            .arg(source_url)
            .arg("--output")
            .arg("embree.tar.gz")
            .output()
            .unwrap();
        std::fs::create_dir_all(&out_dir).unwrap();
        Command::new("tar")
            .args([
                "-zxvf",
                "embree.tar.gz",
                "-C",
                &out_dir,
                "--strip-components=1",
            ])
            .output()
            .unwrap();
    }
}

fn get_out_dir() -> Option<String> {
    println!("cargo:rerun-if-env-changed=EMBREE_DLL_OUT_DIR");
    env::var("EMBREE_DLL_OUT_DIR").ok()
}

fn build_embree_from_source() -> Result<()> {
    let out_dir = build_embree()?;
    gen(&out_dir)?;
    let out_dir = env::var("OUT_DIR").unwrap();
    println!("cargo:rustc-link-search=native={}/bin/", out_dir);
    println!("cargo:rustc-link-search=native={}/lib/", out_dir);
    println!("cargo:rustc-link-lib=dylib=embree4");
    let out_dir = PathBuf::from(out_dir);
    if let Some(dst_dir) = get_out_dir() {
        dbg!(&dst_dir);
        let dst_dir = PathBuf::from(dst_dir);
        fs::create_dir_all(&dst_dir).unwrap();
        assert!(dst_dir.exists());
        let out_dir = fs::canonicalize(out_dir).unwrap();
        let get_dll_dir = |subdir| {
            let dll_dir = out_dir.clone().join(subdir);
            let dll_dir = PathBuf::from(dll_dir);
            assert!(dll_dir.exists());
            fs::canonicalize(dll_dir).unwrap()
        };
        copy_dlls(&get_dll_dir("lib"), &dst_dir);
    }
    Ok(())
}

fn prebuild() -> Result<()> {
    gen(&"embree".to_string())?;
    let cur_file = fs::canonicalize(file!())?;
    let current_dir = cur_file.parent().unwrap();
    println!(
        "cargo:rustc-link-search=native={}/embree/bin/",
        current_dir.display()
    );
    println!(
        "cargo:rustc-link-search=native={}/embree/lib/",
        current_dir.display()
    );
    println!("cargo:rustc-link-lib=dylib=embree4");

    let get_dll_dir = |subdir: &str| {
        let dll_dir = PathBuf::from("embree").join(subdir);
        assert!(dll_dir.exists());
        fs::canonicalize(dll_dir).unwrap()
    };
    if let Some(dst_dir) = get_out_dir() {
        dbg!(&dst_dir);
        let dst_dir = PathBuf::from(dst_dir);
        fs::create_dir_all(&dst_dir).unwrap();
        assert!(dst_dir.exists());
        let dst_dir = fs::canonicalize(dst_dir).unwrap();
        copy_dlls(&get_dll_dir("lib"), &dst_dir);
        if cfg!(target_os = "windows") {
            copy_dlls(&get_dll_dir("bin"), &dst_dir);
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    download_embree();
    if prebuild_available() {
        prebuild()?;
    } else {
        build_embree_from_source()?;
    }
    Ok(())
}
