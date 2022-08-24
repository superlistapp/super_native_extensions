# Native Drag & Drop for Flutter

## Features

- Native Drag&Drop functionality
- Supports macOS, iOS, Android, Windows, Linux and Web (*)
- Platform agnostic support for dragging and dropping common formats
- Support for custom data formats
- Multifinger drag on iOS (adding item to existing drag session)
- Dragging and dropping virtual files (macOS, iOS and Windows)

*) Web supports dropping from other applications, but dragging only works within the same browser tab.

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

### Android support

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

### Dragging from the application

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
        // on drop (optional)
        item.add(Formats.plainText('Plain Text Data'));
        item.add(Formats.htmlText.lazy(() => '<b>HTML generated on demand</b>'));
        return item;
      },
      allowedOperations: () => [DropOperation.copy],
      child: const Text('This widget is draggable'),
    );
  }
}
```

This widget will create a draggable area. `dragItemProvider` callback is invoked every time user attempts to drag the widget. The drag gesture is platform and device specific. If item provider returns `null` drag will not start.

`DragSession` passed into the item provider contains listenables that can be used to monitor the state of the drag session.

### Receiving dragged items

```dart
class MyDropRegion extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return DropRegion(
      // Formats this region can accept
      formats: Formats.standardFormats,
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (session, position) {
        // You can inspect local data here, as well as formats of each item.
        // However on certain platforms (mobile / web) the actual data is
        // only available when the drop is accepted.
        final item = session.items.first;
        if (item.localData is Map) {
          // This is a drag within the app and has custom local data set
        }
        if (item.hasValue(Formats.plainText)) {
          // this item contains plain text
        }
        // This drop region only supports copy operation
        if (session.allowedOperations.contains(DropOperation.copy)) {
          return DropOperation.copy;
        } else {
          return DropOperation.none;
        }
      },
      onDropEnter: (session) {
        // This is called when region first accepts a drag. You can use this
        // to display a visual indicator that the drop is allowed.
      },
      onDropLeave: (session) {
        // Called when drag leaves the region. Will also be called after
        // drag completion.
        // This is a good place to remove any visual indicators.
      },
      onPerformDrop: (session, position, acceptedOperation) async {
        // Called when user dropped the item. You can now request the data.
        // Note that data must be requested before the performDrop callback
        // is over.
        final item = session.items.first;
        // data reader is available now
        final reader = item.dataReader!;
        if (reader.hasValue(Formats.plainText)) {
          reader.getValue(Formats.plainText, (value) {
            if (value.error != null) {
              print('Error reading value ${value.error}');
            } else {
              print('Dropped text: ${value.value}');
            }
          });
        }
      },
      child: const Padding(
        padding: EdgeInsets.all(15.0),
        child: Text('Drop items here'),
      ),
    );
  }
}
```

One desktop platforms full drag data is available in `onDropOver`. On mobile and web platforms, data is only available when the drop is accepted and in `onDropOver` you can only query data format.

Local data is always available.

Note that `getValue` does not return a promise, instead it uses callback. This is intentional to avoid accidentally blocking `onPerformDrop` by awaiting the `getValue` result. Getting the value
might take a while to complete and `onPerformDrop` will block the platform thread so it must return quickly.

## Advanced usage

### Dragging virtual files

Virtual files are files that do not physically exist at the moment of drag. On drop the application gets notified and will start producing file content. This is useful when dragging content that is displayed in application but actually exist on a remote location (cloud).

```dart
// TODO(knopp): Example
```

### Dragging multiple items on iPad

```dart
// TODO(knopp): Example
```

### Dropping virtual files

```dart
// TODO(knopp): Example
```

This plugin is in a very early stages of development and quite experimental.

Example project is available at `super_drag_and_drop/example`.

PRs and bug reports are welcome!
