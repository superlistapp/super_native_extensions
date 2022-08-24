## Features

- Comprehensive clipboard functionality for Flutter.
- Supports macOS, iOS, Android, Windows, Linux and Web.
- Platform agnostic support for reading and writing common clipboard formats.
- Support for custom data formats.
- Multiple representation for clipboard items.
- Providing clipboard data on demand.

<img src="https://matejknopp.com/super_native_extensions/super_clipboard.png"/>

## Getting started

`super_clipboard` uses Rust internally to implement low-level platform functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

To use `super_clipboard`, you will need to install Rust:

For macOS or Linux, execute the following in Terminal.
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
```
For Windows, you can use the [Rust Installer](https://static.rust-lang.org/rustup/dist/x86_64-pc-windows-msvc/rustup-init.exe).

In case you have Rust already installed, make sure to update it to latest version:

```bash
rustup update
```

That is it. The build integration will automatically install required Rust toolchains and other dependencies. This also means that first build might take a little bit longer.

### Android Support

To be able to write images and other custom data to Android clipboard you need
to declare a content provider in `AndroidManifest.xml`:

```xml
<manifest>
    <application>
        ...
        <provider
            android:name="com.superlist.super_native_extensions.DataProvider"
            android:authorities="<your-package-name>.ClipboardDataProvider"
            android:exported="true"
            android:grantUriPermissions="true" >
        </provider>
        ...
    </application>
</manifest>
```
Be sure to replace `<your-package-name>` in the snipped with your actual package name.

## Usage

### Reading from Clipboard

```dart
    final reader = await ClipboardReader.readClipboard();
    if (reader.hasValue(Format.htmlText)) {
        final html = await reader.readValue(Format.htmlText);
        // .. do something with the HTML text
    }
    if (reader.hasValue(Format.plainText)) {
        final text = await reader.readValue(Format.plainText);
        // Do something with the plain text
    }
    if (reader.hasValue(Format.imagePng)) {
        final png = await reader.readValue(Format.imagePng);
        // Do something with the PNG image
    }
```

For more formats supported out of box look at the [Format](lib/src/standard_formats.dart) class.

Note that on Windows clipboard images are usually stored in DIB or DIBv5 format, while on macOS TIFF is commonly used. `super_clipboard` will transparently expose these formats as PNG.

You can query whether the PNG image in clipboard has been synthetized through `reader.isSynthetized(Format.png)`.

### Writing to Clipboard

```dart
    final item = DataWriterItem();
    item.add(Format.htmlText.encode('<b>HTML text</b>'));
    item.add(Format.plainText.encode('plain text'));
    item.add(Format.imagePng.encode(imageData));
    await ClipboardWriter.instance.write([item]);
```

You can also provide representations on demand:

```dart
    final item = DataWriterItem();
    item.add(Format.htmlText.encodeLazy(() => '<b>HTML text</b>'));
    item.add(Format.plainText.encodeLazy(() => 'plain text'));
    item.add(Format.imagePng.encodeLazy(() => imageData));
    await ClipboardWriter.instance.write([item]);
```

If you do this make sure that the callback can provide requested data without any uncecessary delay. On some platforms main thread may be blocked while the data is being requested.

On some platform the data may be requested eagerly when writing to clipboard. In this case the callback will be called immediately.

When writing images preferred format is PNG. Most platform can handle PNG images in clipboard natively. On Windows PNGs are on-demand converted to DIB and DIBv5 formats, which is native applications expect.

While the Clipboard API supports writing multiple items, not all platforms support that fully. On Windows clipboard items past the first one only support `Format.fileUri` type (so it is possible to store multiple File Uris in clipboard) and on Linux only supported formats for additional items are `Format.uri` and `Format.fileUri`.

## Additional information

This plugin is in a very early stages of development and quite experimental.

Example project is available at `super_clipboard/example`.

PRs and bug reports are welcome!
