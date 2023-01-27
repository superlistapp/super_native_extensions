# Clipboard access for Flutter

## Features

- Comprehensive clipboard functionality for Flutter.
- Supports macOS, iOS, Android, Windows, Linux and Web.
- Platform agnostic code for reading and writing common clipboard formats.
- Support for custom data formats.
- Multiple representation for clipboard items.
- Providing clipboard data on demand.

<img src="https://matejknopp.com/super_native_extensions/super_clipboard.png" width="831"/>

## Getting started

`super_clipboard` uses Rust internally to implement low-level platform specific functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

To use `super_clipboard`, you will need to install Rust:

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

### Android support

NDK is required to use `super_clipboard`. You can use Android Studio *SDK Manager* to install the NDK:

    Preferences -> Android SDK -> SDK Tools -> NDK (Side by Side).

NDK version your project expects is specified in `build.gradle`.

```groovy
android {
    // by default the project uses NDK version from flutter plugin.
    ndkVersion flutter.ndkVersion
```
You can find the current value of `flutter.ndkVersion` in Flutter source code ([stable](https://github.com/flutter/flutter/blob/stable/packages/flutter_tools/gradle/flutter.gradle), [master](https://github.com/flutter/flutter/blob/master/packages/flutter_tools/gradle/flutter.gradle)).

```java
class FlutterExtension {
    // ...
    static String ndkVersion = ....
```

To be able to write images and other custom data to Android clipboard you need
to declare a content provider in `AndroidManifest.xml`:

```xml
<manifest>
    <application>
        ...
        <provider
            android:name="com.superlist.super_native_extensions.DataProvider"
            android:authorities="<your-package-name>.SuperClipboardDataProvider"
            android:exported="true"
            android:grantUriPermissions="true" >
        </provider>
        ...
    </application>
</manifest>
```
Be sure to replace `<your-package-name>` in the snippet with your actual package name.

## Usage

### Reading from clipboard

```dart
    final reader = await ClipboardReader.readClipboard();

    if (reader.canProvide(Formats.htmlText)) {
        final html = await reader.readValue(Formats.htmlText);
        // .. do something with the HTML text
    }

    if (reader.canProvide(Formats.plainText)) {
        final text = await reader.readValue(Formats.plainText);
        // Do something with the plain text
    }

    /// Binary formats need to be read as streams
    if (reader.canProvide(Formats.png)) {
        reader.getFile(Formats.png, (result) {
            // Do something with the PNG image
            final stream = result.value?.getStream();
        });
    }
```

For more formats supported out of box look at the [Formats](https://github.com/superlistapp/super_native_extensions/blob/main/super_clipboard/lib/src/standard_formats.dart) class.

Note that on Windows clipboard images are usually stored in DIB or DIBv5 format, while on macOS TIFF is commonly used. `super_clipboard` will transparently expose these images as PNG.

You can query whether the PNG image in clipboard has been synthetized through `reader.isSynthetized(Formats.png)`.

### Writing to clipboard

```dart
    final item = DataWriterItem();
    item.add(Formats.htmlText('<b>HTML text</b>'));
    item.add(Formats.plainText('plain text'));
    item.add(Formats.png(imageData));
    await ClipboardWriter.instance.write([item]);
```

You can also provide representations on demand:

```dart
    final item = DataWriterItem();
    item.add(Formats.htmlText.lazy(() => '<b>HTML text</b>'));
    item.add(Formats.plainText.lazy(() => 'plain text'));
    item.add(Formats.png.lazy(() => imageData));
    await ClipboardWriter.instance.write([item]);
```

If you do this make sure that the callback can provide requested data without any uncecessary delay. On some platforms main thread may be blocked while the data is being requested. This functionality is meant for providing alternative representations on demand. Do **not** start downloading a file from lazy callback or any other action that is not guaranteed to complete in short time. For copying or dragging files that are not readily available use `DataWriterItem.addVirtualFile` instead.

On some platform the data may be requested eagerly when writing to clipboard. In this case the callback will be called immediately.

When writing images preferred format is PNG. Most platform can handle PNG images in clipboard natively. On Windows PNGs are on-demand converted to DIB and DIBv5 formats, which is what native applications expect.

While the Clipboard API supports writing multiple items, not all platforms support that fully. On Windows clipboard items past the first one only support `Formats.fileUri` type (so it is possible to store multiple file URIs in clipboard) and on Linux only supported formats for additional items are `Formats.uri` and `Formats.fileUri`.

## Running the example

Example project is available at `super_clipboard/example`.

```bash
flutter pub global activate melos # if you don't have melos already installed
git clone https://github.com/superlistapp/super_native_extensions.git
cd super_native_extensions
melos bootstrap
```

After this you can open the folder in VSCode and run the `clipboard_example` launcher configuration.

TODO(knopp): Add Intellij launcher configuration

## Additional information

This plugin is in a very early stages of development and quite experimental.

[PRs](https://github.com/superlistapp/super_native_extensions/pulls) and [bug reports](https://github.com/superlistapp/super_native_extensions/issues) are welcome!
