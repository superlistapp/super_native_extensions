use anyhow::Result;
use log::info;
use std::{env::Args, path::PathBuf, process::Command};

use super::utils::*;

fn get_archs() -> Vec<String> {
    let archs = std::env::var("TOOLBOX_ARCHS")
        .ok()
        .unwrap_or_else(|| "x86_64".into());
    archs.split(' ').into_iter().map(String::from).collect()
}

fn target_for_arch(arch: &str) -> Result<String> {
    let platform_name = string_from_env("TOOLBOX_PLATFORM_NAME")?;
    let suffix = match platform_name.as_str() {
        "macosx" => "apple-darwin",
        "iphonesimulator" => "apple-ios-sim",
        "iphoneos" => "apple-ios",
        platform => {
            return Err(anyhow::format_err!("Unknown platform {}", platform));
        }
    };
    let arch = match arch {
        "arm64" => "aarch64",
        "armv7" => "armv7",
        "x86_64" => "x86_64",
        arch => {
            return Err(anyhow::format_err!("Unknown architecture {}", arch));
        }
    };
    let res = format!("{}-{}", arch, suffix);
    if res == "x86_64-apple-ios-sim" {
        return Ok("x86_64-apple-ios".into());
    }
    Ok(res)
}

fn is_release() -> bool {
    let configuration = std::env::var("TOOLBOX_CONFIGURATION")
        .ok()
        .unwrap_or_else(|| "Release".into());
    configuration != "Debug"
}

fn manifest_path(src_path: &str) -> Result<PathBuf> {
    let src_root = path_from_env("TOOLBOX_SRCROOT")?;
    // Resolve symlink so that crates with relative paths work properly
    // when using as flutter plugin (which itself is symlinked)
    Ok(src_root.join(src_path).join("Cargo.toml").canonicalize()?)
}

fn temp_target_dir() -> Result<PathBuf> {
    let target_dir = string_from_env("TOOLBOX_TEMP_DIR")?;
    Ok(target_dir.into())
}

fn final_target_dir() -> Result<PathBuf> {
    let product_name = string_from_env("TOOLBOX_PRODUCT_NAME")?;
    let target_path = path_from_env("TOOLBOX_TARGET_DIR")?;
    Ok(target_path.join(product_name))
}

pub fn build_pod(mut args: Args) -> Result<()> {
    let src_path = args.next().unwrap();
    let lib_name = args.next().unwrap();

    let archs = get_archs();
    let targets = installed_targets()?;
    for arch in &archs {
        let target = target_for_arch(arch)?;
        if !targets.contains(&target) {
            info!("Installing rust target {}...", target);
            install_target(&target)?;
        }
    }

    for arch in &archs {
        let mut cmd = Command::new("cargo");
        cmd.arg("build");
        cmd.arg("--manifest-path");
        cmd.arg(manifest_path(&src_path)?);
        cmd.arg("-p");
        cmd.arg(&lib_name);
        if is_release() {
            cmd.arg("--release");
        }
        cmd.arg("--target");
        cmd.arg(target_for_arch(arch)?);
        cmd.arg("--target-dir");
        cmd.arg(temp_target_dir()?);
        run_command(cmd)?;
    }

    {
        let lib_name = format!("lib{}.a", lib_name);
        let mut cmd = Command::new("lipo");
        cmd.arg("-create");
        for arch in &archs {
            let path = temp_target_dir()?
                .join(target_for_arch(arch)?)
                .join(if is_release() { "release" } else { "debug" })
                .join(&lib_name);
            cmd.arg(path);
        }
        cmd.arg("-output");
        cmd.arg(final_target_dir()?.join(lib_name));
        run_command(cmd)?;
    }

    Ok(())
}
