Native Drag&Drop for Flutter

## Features

- Native Drag&Drop functionality
- Supports macOS, iOS, Android, Windows, Linux and Web
- Platform agnostic support for dragging and dropping common formats
- Support for custom data formats
- Multifinger drag on iOS (adding item to existing drag session)
- Dragging and dropping virtual files (macOS, iOS and Windows)

## Getting started

`super_drag_and_drop`

ses Rust internally to implement low-level platform functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

To use `super_drag_and_drop`, you will need to install Rust:

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

### Android Support

To be able to drag images and other custom data from your application you need
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
Be sure to replace `<your-package-name>` in the snippet with your actual package name. Note that this is same content provider as the one `super_clipboard` uses. If you are using both packages, you only need to do this once.

## Usage

### Dragging from the Application

```dart
class MyDraggableWidget extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      dragItemProvider: (snapshot, session) async {
        final item = DragItem(
          // snapshot() will return image snapshot  of the DragItemWidget.
          // You can use any other drag image if your wish
          image: await snapshot(),
          // This data is only accessible when dropping within same
          // application (optional)
          localData: {'x': 3, 'y': 4},
        );
        // Add data for this item that other applications can read
        // on Drop (optional)
        item.add(Formats.plainText('Plain Text Data'));
        return item;
      },
      allowedOperations: () => [DropOperation.copy],
      child: const Text('This widget is draggable'),
    );
  }
}
```

This widget will create a draggable area. `dragItemProvider` callback is invoked every time user attempts to drag the widget. The gesture is platform and device specific. If item provider returns null dragging will be disabled.

`DragSession` passed into the item provider contains listenables that can be used to monitor the state of the drag session.


