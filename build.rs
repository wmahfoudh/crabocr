use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let out_dir = env::var("OUT_DIR").unwrap();
    let vendor_dir = PathBuf::from(&manifest_dir).join("vendor").join("mupdf-1.23.11-source");

    println!("cargo:rerun-if-changed=wrapper_mupdf.h");
    println!("cargo:rerun-if-changed={}", vendor_dir.display());

    // 1. Build MuPDF
    // Build MuPDF with release configuration and disabled external dependencies.
    let build_dir = vendor_dir.join("build/release");
    
    let status = Command::new("make")
        .current_dir(&vendor_dir)
        .arg("build=release")
        .arg(format!("OUT={}", build_dir.display()))
        .arg("HAVE_X11=no")
        .arg("HAVE_GLUT=no")
        .arg("HAVE_CURL=no")
        .arg("HAVE_WAYLAND=no") // just in case
        .status()
        .expect("Failed to execute make for mupdf");

    if !status.success() {
        panic!("MuPDF make failed");
    }

    // 2. Link MuPDF
    // The libs are usually in build/release/
    let build_dir = vendor_dir.join("build/release");
    
    println!("cargo:rustc-link-search=native={}", build_dir.display());
    println!("cargo:rustc-link-lib=static=mupdf");
    println!("cargo:rustc-link-lib=static=mupdf-third");
    
    // Link MuPDF and its third-party dependencies.

    // 2.5 Compile C Wrapper
    cc::Build::new()
        .file("src/wrapper.c")
        .include(vendor_dir.join("include"))
        .compile("mupdf_wrapper");
    println!("cargo:rerun-if-changed=src/wrapper.c");
    println!("cargo:rerun-if-changed=src/wrapper.h");

    // 3. Build Leptonica
    let lept_dst = cmake::Config::new("vendor/leptonica-1.83.1")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_PROG", "OFF")
        .define("BUILD_EXAMPLE", "OFF")
        .define("SW_BUILD", "OFF")
        .define("CMAKE_POLICY_VERSION_MINIMUM", "3.5")
        // Disable image format support to avoid external dependencies.
        .define("LIBWEBP_SUPPORT", "OFF")
        .define("OPENJPEG_SUPPORT", "OFF")
        .define("CMAKE_DISABLE_FIND_PACKAGE_GIF", "TRUE")
        .define("CMAKE_DISABLE_FIND_PACKAGE_PNG", "TRUE")
        .define("CMAKE_DISABLE_FIND_PACKAGE_TIFF", "TRUE")
        .define("CMAKE_DISABLE_FIND_PACKAGE_ZLIB", "TRUE")
        .define("CMAKE_DISABLE_FIND_PACKAGE_PkgConfig", "TRUE") // Prevent finding system libraries.
        .build();

    println!("cargo:rustc-link-search=native={}", lept_dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=leptonica");

    // 4. Build Tesseract
    let tess_dst = cmake::Config::new("vendor/tesseract-5.3.4")
        .define("BUILD_SHARED_LIBS", "OFF")
        .define("BUILD_TRAINING_TOOLS", "OFF")
        .define("BUILD_TESTS", "OFF")
        .define("ENABLE_LTO", "OFF")
        .define("Leptonica_DIR", lept_dst.join("lib/cmake/leptonica"))
        .define("CMAKE_PREFIX_PATH", lept_dst)
        .define("ENABLE_CURL", "OFF")
        .define("ENABLE_TIFF", "OFF")
        .define("DISABLE_ARCHIVE", "ON")
        .define("DISABLE_CURL", "ON")
        .define("DISABLE_TIFF", "ON")
        .build();

    println!("cargo:rustc-link-search=native={}", tess_dst.join("lib").display());
    println!("cargo:rustc-link-lib=static=tesseract");
    println!("cargo:rustc-link-lib=stdc++"); // Tesseract is C++

    // 5. Generate Tesseract Bindings
    
    let tess_bindings = bindgen::Builder::default()
        .header(tess_dst.join("include/tesseract/capi.h").to_str().unwrap())
        .clang_arg(format!("-I{}", tess_dst.join("include").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .allowlist_function("Tess.*")
        .generate()
        .expect("Unable to generate Tesseract bindings");

    let tess_out_path = PathBuf::from(&out_dir).join("bindings_tesseract.rs");
    tess_bindings
        .write_to_file(tess_out_path)
        .expect("Couldn't write Tesseract bindings!");

    // Existing MuPDF bindings generation...
    let bindings = bindgen::Builder::default()
        .header("src/wrapper.h")
        .clang_arg(format!("-I{}", vendor_dir.join("include").display()))
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        // Allow listed functions and types
        .allowlist_function("my_.*")
        .allowlist_type("fz_.*") // We need fz_context etc.
        .generate()
        .expect("Unable to generate bindings");

    let out_path = PathBuf::from(out_dir).join("bindings_mupdf.rs");
    bindings
        .write_to_file(out_path)
        .expect("Couldn't write bindings!");
        
    // Link the standard math library.
    println!("cargo:rustc-link-lib=m"); 
}
