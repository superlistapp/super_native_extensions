@echo off
setlocal

set Path=%HOME%\.cargo\bin;%Path%

SET BASEDIR=%~dp0

WHERE cargo

if errorlevel 1 (
    echo ================================================================
    echo ==
    echo == cargo not found!
    @REM echo ==
    @REM echo == We looked in Path: "%Path%"
    echo ==
    echo == Maybe you need to install Rust? It only takes a minute:
    echo ==
    echo == https://www.rust-lang.org/tools/install
    echo ==
    echo ================================================================
    exit 1
)

cargo run --manifest-path=%BASEDIR%/build_tool/Cargo.toml --bin build_tool --target-dir=%CARGOKIT_TOOL_TEMP_DIR% -- %*
