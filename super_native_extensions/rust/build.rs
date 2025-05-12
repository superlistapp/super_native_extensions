#[path = "keyboard_map/gen_keyboard_map.rs"]
mod gen_keyboard_map;

fn main() {
    let target_system = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    let target_arch = std::env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    // keyboard map is currently only supported on desktop.
    match target_system.as_str() {
        "macos" | "linux" | "windows" => {
            gen_keyboard_map::generate_keyboard_map(&target_system).unwrap();
        }
        _ => {}
    }

    // Add support for the maximum page size of 16 KB for Android (only on the arm64 and x86_64 platforms).
    // see: https://developer.android.com/guide/practices/page-sizes#other-build-systems
    if target_system == "android" && (target_arch == "aarch64" || target_arch == "x86_64") {
        println!("cargo:rustc-link-arg=-Wl,-z,max-page-size=16384");
    }
}
