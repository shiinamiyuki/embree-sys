use sha2::{Digest, Sha256};
use std::io::Result;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};
fn build_embree() -> Result<String> {
    let out_dir = env::var("OUT_DIR").unwrap();
    let generator = env::var("CMAKE_GENERATOR").unwrap_or("Ninja".to_string());

    let mut build = cmake::Config::new("embree");
    build
        .generator(generator)
        .define("CMAKE_BUILD_TYPE", "Release")
        .define("EMBREE_ISPC_SUPPORT", "OFF")
        .define("EMBREE_TASKING_SYSTEM", "INTERNAL")
        .define("EMBREE_TUTORIALS", "OFF")
        .define("EMBREE_STATIC_LIB", "OFF")
        .define("EMBREE_GEOMETRY_QUAD", "OFF")
        .define("EMBREE_GEOMETRY_CURVE", "ON")
        .define("EMBREE_GEOMETRY_SUBDIVISION", "OFF")
        .define("EMBREE_RAY_MASK", "ON")
        .define("EMBREE_GEOMETRY_POINT", "OFF")
        .define("CMAKE_MACOSX_RPATH", "ON")
        .define("CMAKE_SKIP_BUILD_RPATH", "OFF")
        .define("CMAKE_BUILD_RPATH_USE_ORIGIN", "ON")
        .define("CMAKE_BUILD_WITH_INSTALL_RPATH", "ON")
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5") // workaround CMake 4.0
        .define(
            "CMAKE_INSTALL_RPATH",
            if cfg!(target_os = "linux") {
                "$ORIGIN"
            } else if cfg!(target_os = "macos") {
                "@loader_path"
            } else {
                ""
            },
        )
        .define(
            "EMBREE_ARM",
            if cfg!(target_arch = "x86_64") {
                "OFF"
            } else {
                "ON"
            },
        )
        .define(
            "EMBREE_MAX_ISA",
            if cfg!(target_arch = "x86_64") {
                "AVX2"
            } else {
                "NEON2X"
            },
        );
    match env::var("EMBREE_CC") {
        Ok(cc) => {
            build.define("CMAKE_C_COMPILER", cc);
        }
        Err(_) => {}
    }
    match env::var("EMBREE_CXX") {
        Ok(cxx) => {
            build.define("CMAKE_CXX_COMPILER", cxx);
        }
        Err(_) => {}
    }
    build.build();
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
        && !path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("msvcp")
        && !path
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .contains("vcruntime")
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
    if !src_dir.exists() {
        return;
    }
    let src_dir = fs::canonicalize(src_dir).unwrap();
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
    println!("cargo:rerun-if-env-changed=EMBREE_FORCE_BUILD_FROM_SOURCE");
    let force_build = env::var("EMBREE_FORCE_BUILD_FROM_SOURCE").unwrap_or("0".to_string());
    let force_build = force_build.to_uppercase();
    let force_build = force_build == "1" || force_build == "ON" || force_build == "TRUE";
    if force_build {
        return false;
    }
    if cfg!(target_arch = "x86_64") && (cfg!(target_os = "windows") || cfg!(target_os = "linux")) {
        true
    } else {
        false
    }
}

fn sha256sum(filename: &str) -> String {
    let mut file = std::fs::File::open(filename).unwrap();
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).unwrap();
    let hash = hasher.finalize();
    format!("{:X}", hash)
}

fn download_with_curl(url: &str, output: &str, expected_hash: &str) {
    match env::var("EMBREE_ZIP_FILE") {
        Ok(path) => {
            if !path.is_empty() {
                let hash = sha256sum(&path);
                assert_eq!(
                    hash, expected_hash,
                    "You have downloaded wrong `{}`, expected hash {} but found {}\nPlease download again from {}",
                    output, expected_hash, hash, url
                );
                if &path != output {
                    std::fs::copy(&path, output).unwrap();
                }
                return;
            }
        }
        Err(_) => {}
    }
    eprintln!("Downloading embree...");
    let mut curl = Command::new("curl")
        .arg("-L")
        .arg(url)
        .arg("--output")
        .arg(output)
        .arg("-m")
        .arg("300")
        .spawn()
        .unwrap();
    let exit_status = curl.wait().unwrap();
    if !exit_status.success() {
        panic!("Unable to download embree");
    }
    let code = exit_status.code();
    if let Some(code) = code {
        if code == 28 {
            eprintln!("Unable to download embree, timeout");
            panic!("Unable to download embree, timeout");
        } else if code != 0 {
            eprintln!("Unable to download embree, exit code {}", code);
            panic!("Unable to download embree, exit code {}", code);
        }
    } else {
        eprintln!("Unable to download embree, exit code unknown");
        panic!("Unable to download embree, exit code unknown");
    }
    let hash = sha256sum(output);
    assert_eq!(
        hash.to_lowercase(), expected_hash.to_lowercase(),
        "File corrupted, expected hash {} for `{}` but found {}",
        expected_hash, output, hash
    );
}
fn download_embree() {
    let linux_url = r#"https://github.com/RenderKit/embree/releases/download/v4.4.0/embree-4.4.0.x86_64.linux.tar.gz"#;
    let windows_url = r#"https://github.com/RenderKit/embree/releases/download/v4.4.0/embree-4.4.0.x64.windows.zip"#;
    let source_url = r#"https://github.com/RenderKit/embree/archive/refs/tags/v4.4.0.tar.gz"#;
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
        let hash = if cfg!(target_os = "windows") {
            "d951e5e6bd295c54cdd66be9cdb44a4e8c42fb38a99f94f79305e48765fc3454"
        } else {
            "cb3d4402537fc9165c76c3316b8953dcfea523cd1eaf588e2de7639864ee3c57"
        };
        download_with_curl(url, filename, hash);
        std::fs::create_dir_all(&out_dir).unwrap();
        if cfg!(target_os = "windows") {
            Command::new("tar")
                .args(["-zxvf", filename, "-C", &out_dir])
                .output()
                .unwrap();
        } else {
            Command::new("tar")
                .args(["-zxvf", filename, "-C", &out_dir])
                .output()
                .unwrap();
        }
    } else {
        download_with_curl(
            source_url,
            "embree.tar.gz",
            "acb517b0ea0f4b442235d5331b69f96192c28da6aca5d5dde0cbe40799638d5c",
        );
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
    println!("cargo:rustc-link-search=native={}/lib64/", out_dir);
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
            PathBuf::from(dll_dir)
        };
        copy_dlls(&get_dll_dir("lib"), &dst_dir);
        copy_dlls(&get_dll_dir("lib64"), &dst_dir);
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
    println!("cargo:rerun-if-env-changed=EMBREE_ZIP_FILE");
    download_embree();
    if prebuild_available() {
        prebuild()?;
    } else {
        build_embree_from_source()?;
    }
    Ok(())
}
