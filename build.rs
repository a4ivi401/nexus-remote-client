use std::env;

fn main() {
    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "macos" {
        // Add rpath for the official GStreamer macOS framework so dyld can find the dylibs at runtime
        println!("cargo:rustc-link-arg=-Wl,-rpath,/Library/Frameworks/GStreamer.framework/Versions/1.0/lib");
    }
}
