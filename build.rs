use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        // Add rpath for Homebrew (ARM & x86) and official GStreamer macOS framework
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Frameworks/GStreamer.framework/Versions/1.0/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/opt/homebrew/lib");
        println!("cargo:rustc-link-arg=-Wl,-rpath,/usr/local/lib");
    }
}
