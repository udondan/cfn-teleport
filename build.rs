fn main() {
    // Only compile Windows resources on Windows targets
    #[cfg(target_os = "windows")]
    {
        compile_windows_resources();
    }

    // On other platforms, this build script does nothing
    // But we still set up rerun triggers for consistency
    #[cfg(not(target_os = "windows"))]
    {
        println!("cargo:rerun-if-changed=build.rs");
    }
}

#[cfg(target_os = "windows")]
fn compile_windows_resources() {
    let mut res = winres::WindowsResource::new();

    // Set version information
    res.set("FileVersion", env!("CARGO_PKG_VERSION"));
    res.set("ProductVersion", env!("CARGO_PKG_VERSION"));

    // Set application information
    res.set("FileDescription", env!("CARGO_PKG_DESCRIPTION"));
    res.set("ProductName", env!("CARGO_PKG_NAME"));
    res.set("CompanyName", env!("CARGO_PKG_AUTHORS"));
    res.set("OriginalFilename", "cfn-teleport.exe");

    // Set copyright information (no year to avoid manual updates)
    let copyright = format!(
        "Copyright © {}. Licensed under {}",
        env!("CARGO_PKG_AUTHORS"),
        env!("CARGO_PKG_LICENSE")
    );
    res.set("LegalCopyright", &copyright);

    // Compile resources and fail build if compilation fails
    res.compile().unwrap_or_else(|e| {
        eprintln!("Failed to compile Windows resources: {}", e);
        std::process::exit(1);
    });

    // Rerun build script if Cargo.toml changes (version updates)
    println!("cargo:rerun-if-changed=Cargo.toml");
}
