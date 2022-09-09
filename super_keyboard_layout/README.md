# Keyboard layout mapping for Flutter

## Features

- Supports macOS, Windows and Linux
- Allows mapping between physical keys, logical keys and platform specific key codes according to current keyboard layout.
- Notification for keyboard layout changes.

<img src="https://user-images.githubusercontent.com/96958/189333008-04ac239e-07d9-49c1-90ef-6bfe4bd4c86d.png" width="762">

## Getting started

`super_keyboard_layout` uses Rust internally to implement low-level platform specific functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

To use `super_keyboard_layout`, you will need to install Rust:

For macOS or Linux, execute the following command in Terminal.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
For Windows, you can use the [Rust Installer](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe).

In case you have Rust already installed, make sure to update it to latest version:

```bash
rustup update
```

That is it. The build integration will automatically install required Rust toolchains and other dependencies. This also means that first build might take a little bit longer.

## Usage

```dart

import 'package:super_keyboard_layout/super_keyboard_layout.dart';

void main() async {
    final manager = await KeyboardLayoutManager.instance();
    if (manager.supported) {
        // Running on supported platform
        manager.onLayoutChanged.addListener(() {
            // Keyboard layout changed
            print('Keyboard layout changed');
        });
    }

    final layout = manager.currentLayout;
    // Getting logical key for physical key 1 with shift for current layout
    final logicalKey = layout.getLogicalKeyForPhysicalKey(PhysicalKeyboardKey.digit1, shift: true);

    // Getting physical key for logical key
    final physicalKey = layout.getPhysicalKeyForLogicalKey(LogicalKeyboardKey.keyA);

    // Getting platform spcific key code for either logical or physical key
    final playformCode = layout.getPlatformKeyCode(PhysicalKeyboardKey.digit1);
}

```

## Running the example

Example project is available at `super_keyboard_layout/example`.

```bash
flutter pub global activate melos # if you don't have melos already installed
git clone https://github.com/superlistapp/super_native_extensions.git
cd super_native_extensions
melos bootstrap
```

After this you can open the folder in VSCode and run the `super_keyboard_layout` launcher configuration.

TODO(knopp): Add Intellij launcher configuration

## Additional information

This plugin is in a very early stages of development and quite experimental.

[PRs](https://github.com/superlistapp/super_native_extensions/pulls) and [bug reports](https://github.com/superlistapp/super_native_extensions/issues) are welcome!
