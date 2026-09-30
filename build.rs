fn main() {
    // Determine the compilation target OS (not the host OS)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();

    if target_os == "windows" {
        // Only attempt to compile Windows resources when building on a Windows host
        // (cross-compilation from non-Windows hosts is not supported for resource embedding)
        compile_windows_resources();
    } else {
        // On non-Windows targets, this build script does nothing
        // But we still set up rerun triggers for consistency
        println!("cargo:rerun-if-changed=build.rs");
    }
}

#[cfg(windows)]
fn compile_windows_resources() {
    // Verify we're actually targeting Windows (double-check)
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os != "windows" {
        println!("cargo:warning=Building non-Windows target on Windows host: skipping resource compilation");
        println!("cargo:rerun-if-changed=build.rs");
        return;
    }

    let mut res = winres::WindowsResource::new();

    // Read package metadata at runtime (not compile time) to avoid stale values
    // when build.rs is rerun without being recompiled
    let version = std::env::var("CARGO_PKG_VERSION").expect("CARGO_PKG_VERSION not set");
    let description =
        std::env::var("CARGO_PKG_DESCRIPTION").expect("CARGO_PKG_DESCRIPTION not set");
    let name = std::env::var("CARGO_PKG_NAME").expect("CARGO_PKG_NAME not set");
    let authors = std::env::var("CARGO_PKG_AUTHORS").expect("CARGO_PKG_AUTHORS not set");
    let license = std::env::var("CARGO_PKG_LICENSE").expect("CARGO_PKG_LICENSE not set");

    // Set version information
    res.set("FileVersion", &version);
    res.set("ProductVersion", &version);

    // Set application information
    res.set("FileDescription", &description);
    res.set("ProductName", &name);
    res.set("CompanyName", &authors);
    res.set("OriginalFilename", "cfn-teleport.exe");

    // Set copyright information (no year to avoid manual updates)
    let copyright = format!("Copyright © {}. Licensed under {}", authors, license);
    res.set("LegalCopyright", &copyright);

    // Compile resources and fail build if compilation fails
    res.compile().unwrap_or_else(|e| {
        eprintln!("Failed to compile Windows resources: {}", e);
        std::process::exit(1);
    });

    // Rerun build script if Cargo.toml changes (version updates)
    println!("cargo:rerun-if-changed=Cargo.toml");
}

#[cfg(not(windows))]
fn compile_windows_resources() {
    // On non-Windows hosts, we cannot compile Windows resources
    // Cross-compilation is not supported for resource embedding
    println!("cargo:warning=Cross-compiling to Windows from non-Windows host: Windows resource compilation is not supported");
    println!("cargo:rerun-if-changed=build.rs");
}
