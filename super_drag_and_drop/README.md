# Native Drag and Drop for Flutter

## Features

- Native Drag and Drop functionality
- Supports macOS, iOS, Android, Windows, Linux and Web (*)
- Platform agnostic code for dragging and dropping common formats
- Support for custom data formats
- Multifinger drag on iOS (adding item to existing drag session, see video below)
- Dragging and dropping virtual files (macOS, iOS and Windows)

*) Web supports dropping from other applications, but dragging only works within the same browser tab.

![Drag Drop Example](https://user-images.githubusercontent.com/96958/186485530-3fa7e938-5805-4dcb-bcff-1da73f15ab63.gif)

## Getting started

`super_drag_and_drop` uses Rust internally to implement low-level platform specific functionality. Rather than shipping prebuilt binaries with the plugin, Rust build is seamlessly integrated into the Flutter build process.

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

That is it. The build integration will automatically install required Rust targets and other dependencies (NDK). This also means that first build might take a little bit longer.

### Android support

NDK is required to use `super_drag_and_drop`. If not present it will be automatically installed during first build. The NDK is a large download (~1GB) so it might take a while to install.

The NDK version used is specified in `android/app/build.gradle` of your Flutter project.

```groovy
android {
    // by default the project uses NDK version from flutter plugin.
    ndkVersion flutter.ndkVersion
```

If you have older Flutter android project, you will need to specify reasonably recent minimal SDK version in `android/app/build.gradle`:

```groovy
android {
    defaultConfig {
        minSdkVersion 23
```

To be able to drag images and other custom data from your application you need
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
Be sure to replace `<your-package-name>` in the snippet with your actual package name. Note that this is same content provider as the one `super_clipboard` uses. If you are using both packages, you only need to do this once.

## Usage

### Dragging from the application

```dart
class MyDraggableWidget extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    // DragItemWidget provides the content for the drag (DragItem).
    return DragItemWidget(
      dragItemProvider: (request) async {
        // DragItem represents the content begin dragged.
        final item = DragItem(
          // This data is only accessible when dropping within same
          // application. (optional)
          localData: {'x': 3, 'y': 4},
        );
        // Add data for this item that other applications can read
        // on drop. (optional)
        item.add(Formats.plainText('Plain Text Data'));
        item.add(
            Formats.htmlText.lazy(() => '<b>HTML generated on demand</b>'));
        return item;
      },
      allowedOperations: () => [DropOperation.copy],
      // DraggableWidget represents the actual draggable area. It looks
      // for parent DragItemWidget in widget hierarchy to provide the DragItem.
      child: const DraggableWidget(
        child: Text('This widget is draggable'),
      ),
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
      // Formats this region can accept.
      formats: Formats.standardFormats,
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: (event) {
        // You can inspect local data here, as well as formats of each item.
        // However on certain platforms (mobile / web) the actual data is
        // only available when the drop is accepted (onPerformDrop).
        final item = event.session.items.first;
        if (item.localData is Map) {
          // This is a drag within the app and has custom local data set.
        }
        if (item.canProvide(Formats.plainText)) {
          // this item contains plain text.
        }
        // This drop region only supports copy operation.
        if (event.session.allowedOperations.contains(DropOperation.copy)) {
          return DropOperation.copy;
        } else {
          return DropOperation.none;
        }
      },
      onDropEnter: (event) {
        // This is called when region first accepts a drag. You can use this
        // to display a visual indicator that the drop is allowed.
      },
      onDropLeave: (event) {
        // Called when drag leaves the region. Will also be called after
        // drag completion.
        // This is a good place to remove any visual indicators.
      },
      onPerformDrop: (event) async {
        // Called when user dropped the item. You can now request the data.
        // Note that data must be requested before the performDrop callback
        // is over.
        final item = event.session.items.first;

        // data reader is available now
        final reader = item.dataReader!;
        if (reader.canProvide(Formats.plainText)) {
          reader.getValue<String>(Formats.plainText, (value) {
            if (value != null) {
              // You can access values through the `value` property.
              print('Dropped text: ${value}');
            }
          }, onError: (error) {
            print('Error reading value $error');
          });
        }

        if (reader.canProvide(Formats.png)) {
          reader.getFile(Formats.png, (file) {
            // Binary files may be too large to be loaded in memory and thus
            // are exposed as stream.
            final stream = file.getStream();

            // Alternatively, if you know that that the value is small enough,
            // you can read the entire value into memory:
            // (note that readAll is mutually exclusive with getStream(), you
            // can only use one of them)
            // final data = file.readAll();
          }, onError: (error) {
            print('Error reading value $error');
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

## Data formats

When it comes to providing and receiving drag data, `super_drag_and_drop` builds on top of `super_clipboard`. Please see `super_clipboard` [documentation](https://github.com/superlistapp/super_native_extensions/tree/main/super_clipboard#reading-from-clipboard) for more information about data formats.

## Advanced usage

### Dragging virtual files

Virtual files are files that do not physically exist at the moment of drag. On drop the application gets notified and will start producing file content. This is useful when dragging content that is displayed in application but actually exist on a remote location (cloud).

Virtual files are supported on iOS, macOS and Windows.

```dart
  final item = DragItem();
  item.addVirtualFile(
    format: Formats.plainTextFile,
    provider: (sinkProvider, progress) {
      final line = utf8.encode('Line in virtual file\n');
      const lines = 10;
      final sink = sinkProvider(fileSize: line.length * lines);
      for (var i = 0; i < lines; ++i) {
        sink.add(line);
      }
      sink.close();
    },
  );
```

### Dragging multiple items on iOS

To allow dragging multiple items on iOS (as in the example video above), create the
`DragItemWidget` with `canAddItemToExistingSession` set to `true`.

In the `dragItemProvider` callback, return `null` if if the drag session already
has local drag data associated with the item. Otherwise the item can be added to
session multiple times.

```dart
  return DragItemWidget(
    allowedOperations: () => [DropOperation.copy],
    canAddItemToExistingSession: true,
    dragItemProvider: (request) async {
      // For multi drag on iOS check if this item is already in the session
      if (await request.session.hasLocalData('image-item')) {
        return null;
      }
      final item = DragItem(
        localData: 'image-item',
        suggestedName: 'Green.png',
      );
      item.add(Formats.png(await createImageData(Colors.green)));
      return item;
    }
  );
```

### Receiving virtual files

Receving virtual files doesn't require any special handling. You can consume the content of virtual file just like any other stream:

```dart
  reader.getFile(Formats.png, (file) {
    final Stream<Uint8List> stream = file.getStream();
    // You can now use the stream to read the file content.
  });
})
```

### Customized drag image

The default drag image is simply a snapshot of the `DragItemWidget` child. To customize drag image use the `liftBuilder` and `dragBuilder` properties on `DragItemWidget`.

```dart
  // Add red background when dragging, blue background during lift.
  DragItemWidget(
    // `child` is the child passed to DragItemWidget.
    // You can use it when building the snapshot or ignore it.
    liftBuilder: (context, child) {
      return Container(color: Colors.blue, child: child);
    }
    dragBuilder: (context, child) {
      return Container(color: Colors.red, child: child);
    }
    ...,
  );
```

Custom snapshot can have size that is different (even larger) than the original widget. You can modify the constraints used to layout custom snapshot as well as
the snapshot position relative to original child using a `SnapshotSettings` widget:

```dart
   // Snapshot will add 1px border around the origin widget.
   DragItemWidget(
    dragBuilder: (BuildContext context, Widget child) {
      return SnapshotSettings(
        // To ensure that the snapshot matches the original widget position we
        // need to shift it by -2 pixels (1px border + 1px padding)
        translation: (rect, dragPosition) => const Offset(-2, -2),
        // Inflate the constraints by two pixels to account for border and padding.
        constraintsTransform: (constraints) =>
            constraints.deflate(const EdgeInsets.all(-2)),
        // Decorate the widget by 1px border and 1px padding.
        child: Container(
          decoration: BoxDecoration(
            borderRadius: BorderRadius.circular(5),
            border: Border.all(color: Colors.black, width: 1),
          ),
          padding: const EdgeInsets.all(1),
          child: child,
        ),
      );
    },
    ...
```

## Synthesized files

On desktop platform, dropping files usually puts the file URL or path into the
payload. This is different from mobile and web, where you can receive the actual
file data.

To streamline this, `super_drag_and_drop` will synthesize a file stream for the dropped file path. This way you can always receive the file content as a stream, regardless of platform.

## Running the example

Example project is available at `super_drag_and_drop/example`.

```bash
flutter pub global activate melos # if you don't have melos already installed
git clone https://github.com/superlistapp/super_native_extensions.git
cd super_native_extensions
melos bootstrap
```

After this you can open the folder in VSCode and run the `drag_and_drop_example` launcher configuration.

TODO(knopp): Add Intellij launcher configuration

## Additional information

This plugin is in a very early stages of development and quite experimental.

[PRs](https://github.com/superlistapp/super_native_extensions/pulls) and [bug reports](https://github.com/superlistapp/super_native_extensions/issues) are welcome!
