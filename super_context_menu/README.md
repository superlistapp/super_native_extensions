# Context Menu for Flutter

## Features

- Single context menu widget that works accross all desktop platforms, mobile platforms and web

- Can transition to native drag & drop on Android & iOS

![1-mobile](https://github.com/superlistapp/super_native_extensions/assets/96958/05cf793e-d848-4244-8685-dab4059e3940)

- Native context menu on iOS, macOS and Linux
- Flutter context menu on Android, Windows and Web

![2-desktop](https://github.com/superlistapp/super_native_extensions/assets/96958/858b559a-9674-4bd0-a167-812e304b0c7d)

- Advanced features such as custom lift image, menu and drag preview, deferred menu items (lazy loading) and deferred menu preview

![3-mobile](https://github.com/superlistapp/super_native_extensions/assets/96958/61cd9630-28e9-47b9-b50b-9df53800e2de)

## Getting started

`super_context_menu` uses Rust internally to implement low-level platform specific functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

To use `super_context_menu`, you will need to install Rust:

For macOS or Linux, execute the following command in Terminal.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
For Windows, you can use the [Rust Installer](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe).

In case you have Rust already installed, make sure to update it to latest version:

```bash
rustup update
```

That is it. The build integration will automatically install required Rust targets and other dependencies (NDK). This also means that first build might take a little bit longer.

## Usage

Basic example:

```dart
  return ContextMenuWidget(
    child: const Item(
    child: Text('Base Context Menu'),
    ),
    menuProvider: (_) {
    return Menu(
        children: [
        MenuAction(title: 'Menu Item 2', callback: () {}),
        MenuAction(title: 'Menu Item 3', callback: () {}),
        MenuSeparator(),
        Menu(title: 'Submenu', children: [
            MenuAction(title: 'Submenu Item 1', callback: () {}),
            MenuAction(title: 'Submenu Item 2', callback: () {}),
            Menu(title: 'Nested Submenu', children: [
            MenuAction(title: 'Submenu Item 1', callback: () {}),
            MenuAction(title: 'Submenu Item 2', callback: () {}),
            ]),
        ]),
        ],
    );
    },
);

```
## Running the example project

Example project is available at `super_context_menu/example`.

```bash
flutter pub global activate melos # if you don't have melos already installed
git clone https://github.com/superlistapp/super_native_extensions.git
cd super_native_extensions
melos bootstrap
```

After this you can open the folder in VSCode and run the `super_context_menu_example` launcher configuration.
