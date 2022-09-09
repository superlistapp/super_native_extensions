#[path = "keyboard_map/gen_keyboard_map.rs"]
mod gen_keyboard_map;

fn main() {
    let target_system = std::env::var("CARGO_CFG_TARGET_OS").unwrap();
    // keyboard map is currently only supported on desktop.
    match target_system.as_str() {
        "macos" | "linux" | "windows" => {
            gen_keyboard_map::generate_keyboard_map(&target_system).unwrap();
        }
        _ => {}
    }
}
