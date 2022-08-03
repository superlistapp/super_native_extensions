import 'dart:convert';
import 'dart:typed_data';
import 'dart:ui' hide window;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:super_clipboard/super_clipboard.dart' hide DataProvider;
import 'package:super_native_extensions/raw_drag_drop.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

class DragException implements Exception {
  final String message;
  DragException(this.message);
}

class _DropDelegate implements DropContextDelegate {
  @override
  Future<void> onDropEnded(BaseDropEvent event) async {
    print('Drop ended $event');
  }

  @override
  Future<void> onDropLeave(BaseDropEvent event) async {
    print('Drop leave $event');
  }

  @override
  Future<DropOperation> onDropUpdate(DropEvent event) async {
    print('Drop update $event, ${event.items.first.itemId}');
    return DropOperation.copy;
  }

  @override
  Future<void> onPerformDrop(DropEvent event) async {
    print('Item count ${event.items.length}, ${event.items.first.itemId}');
    final item = event.items[0];
    // final data = await
    final readerItem = item.readerItem;
    if (readerItem != null) {
      final data = readerItem
          .getDataForFormat((await readerItem.getAvailableFormats()).first);
      final a = await data.first;
      print('Have data ${a}');

      final format = (await readerItem.getAvailableFormats()).first;
      final receiver = await readerItem.getVirtualFileReceiver(format: format);
      final res = receiver?.receiveVirtualFile(
        targetFolder: "/Users/Matej/Projects/1",
      );
      if (res != null) {
        final value = res.first;
        final progress = res.second;
        // final progress = readerItem.getDataForFormat(format, onData: (r) {
        //   print("RES $r");
        // });
        progress.fraction.addListener(() {
          print("PROGRESS ${progress.fraction.value}");
        });
        Future.delayed(Duration(seconds: 2), () {
          print('Cancellable ${progress.cancellable}');
          // progress.cancel();
        });
        // final data = await value;
        value.then((value) {
          print("RES $value");
        });
      }
    }

    // print('DATA $data');
    print('Perform drop $event');
    // Future.delayed(Duration(seconds: 2), () {
    // });
  }

  @override
  Future<ItemPreview?> onGetItemPreview(ItemPreviewRequest request) async {
    print(
        'Item preview ${request.itemId} ${request.size} (${request.fadeOutDelay} ${request.fadeOutDuration})');

    final recorder = PictureRecorder();
    final canvas = Canvas(recorder);
    canvas.drawRect(
        const Rect.fromLTWH(0, 0, 100, 100), Paint()..color = Colors.blue);
    final picture = recorder.endRecording();
    final image = await picture.toImage(100, 100);

    return ItemPreview(
      destinationRect: const Rect.fromLTWH(100, 100, 100, 100),
      destinationImage: await ImageData.fromImage(image),
      // fadeOutDuration: const Duration(milliseconds: 400),
      // fadeOutDelay: const Duration(milliseconds: 100),
    );
  }
}

class _DragDelegate implements DragContextDelegate {
  @override
  bool isLocationDraggable(Offset location) {
    final rect = dragContainer.currentState!.getGlobalRect();
    print('Offset $location');
    return rect.contains(location);
    // return location.dx < 100 && location.dy < 100;
  }

  @override
  Future<DragConfiguration?> getConfigurationForDragRequest(
      {required Offset location, required DragSession session}) async {
    session.dragStarted.addListener(() {
      print('Drag started');
    });
    session.dragCompleted.addListener(() {
      print("Drag completed ${session.dragCompleted.value}");
    });
    // session.sessionIsDoneWithDataSource.addListener(() {
    //   print("Session is done with data source");
    // });
    session.lastScreenLocation.addListener(() {
      print('Last screen location ${session.lastScreenLocation.value}');
    });
    final data = raw.DataProvider(suggestedName: "File1.txt", representations: [
      // DataRepresentation.lazy(
      //     format: 'public.utf8-plain-text',
      //     dataProvider: () => 'plain text lazy'),
      raw.DataRepresentation.virtualFile(
          format: 'public.utf8-plain-text',
          storageSuggestion: raw.VirtualFileStorage.temporaryFile,
          virtualFileProvider: (sinkProvider, progress) async {
            final sink = sinkProvider(fileSize: 32);
            final cancelled = [false];
            print('Requested file');
            progress.onCancel.addListener(() {
              print('Cancelled');
              cancelled[0] = true;
            });
            for (var i = 0; i < 10; ++i) {
              Future.delayed(Duration(milliseconds: i * 1000), () {
                if (cancelled[0]) {
                  return;
                }
                progress.updateProgress(i.toDouble() / 10.0);
                if (i == 9) {
                  print('Done');
                  sink.add(utf8.encode('Hello, cruel world!\n'));
                  sink.add(utf8.encode('Hello, cruel world!'));
                  // sink.addError('Something went wrong');
                  sink.close();
                }
              });
            }
          }),
    ]);
    return DragConfiguration(
      allowedOperations: [DropOperation.copy, DropOperation.move],
      items: [
        DragItem(
          dataProvider: await data.register(),
          localData: {
            'x': 10,
            'abc': 'xyz',
          },
          image:
              await dragContainer.currentState!.getDragImageForOffset(location),
        )
      ],
    );
  }
}

class DragContainer extends StatefulWidget {
  const DragContainer({
    Key? key,
    required this.child,
  }) : super(key: key);
  final Widget child;
  @override
  State<StatefulWidget> createState() => DragContainerState();
}

class DragContainerState extends State<DragContainer> {
  @override
  Widget build(BuildContext context) => widget.child;

  Future<DragImage> getDragImageForOffset(Offset globalPosition) async {
    final renderObject_ = context.findRenderObject();
    final renderObject = renderObject_ is RenderRepaintBoundary
        ? renderObject_
        : context.findAncestorRenderObjectOfType<RenderRepaintBoundary>();
    final pr = MediaQuery.of(context).devicePixelRatio;
    if (renderObject == null) {
      throw DragException("Couldn't find any repaint boundary ancestor");
    }
    final snapshot = await renderObject.toImage(pixelRatio: pr);
    final transform = renderObject.getTransformTo(null);
    final rect = MatrixUtils.transformRect(transform,
        Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));
    final imageData = await ImageData.fromImage(snapshot, devicePixelRatio: pr);
    return DragImage(imageData: imageData, sourceRect: rect);
  }

  Rect getGlobalRect() {
    final renderObject_ = context.findRenderObject();
    final renderObject = renderObject_ is RenderRepaintBoundary
        ? renderObject_
        : context.findAncestorRenderObjectOfType<RenderRepaintBoundary>();
    if (renderObject == null) {
      throw DragException("Couldn't find any repaint boundary ancestor");
    }
    final transform = renderObject.getTransformTo(null);
    final rect = MatrixUtils.transformRect(transform,
        Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));
    return rect;
  }
}

final dragContainer = GlobalKey<DragContainerState>();

void main() async {
  // myKey(20);

  final dropContext = await DropContext.instance();
  await dropContext.registerDropTypes([
    'public.file-url',
    'NSFilenamesPboardType',
    'public.url',
    'public.utf8-plain-text',
    'Apple URL pasteboard type',
    'text/uri-list',
    'text/plain',
  ]);
  await DragContext.instance();
  await DropContext.instance();
  (await DragContext.instance()).delegate = _DragDelegate();
  (await DropContext.instance()).delegate = _DropDelegate();
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({Key? key}) : super(key: key);

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Demo',
      theme: ThemeData(
        // This is the theme of your application.
        //
        // Try running your application with "flutter run". You'll see the
        // application has a blue toolbar. Then, without quitting the app, try
        // changing the primarySwatch below to Colors.green and then invoke
        // "hot reload" (press "r" in the console where you ran "flutter run",
        // or simply save your changes to "hot reload" in a Flutter IDE).
        // Notice that the counter didn't reset back to zero; the application
        // is not restarted.
        primarySwatch: Colors.blue,
      ),
      home: const MyHomePage(title: 'Flutter Clipboard Demo Page'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({Key? key, required this.title}) : super(key: key);

  // This widget is the home page of your application. It is stateful, meaning
  // that it has a State object (defined below) that contains fields that affect
  // how it looks.

  // This class is the configuration for the state. It holds the values (in this
  // case the title) provided by the parent (in this case the App widget) and
  // used by the build method of the State. Fields in a Widget subclass are
  // always marked "final".

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

final formatCustom =
    CustomDataFormat<Uint8List>("com.superlist.clipboard.Example.CustomType");

class _MyHomePageState extends State<MyHomePage> {
  void copy() async {
    // final transfer = DataTransfer();
    final item = ClipboardWriterItem();
    item.addData(formatHtml.encode('<b><i>Html</i></b> Value'));
    item.addData(formatPlainText.encode('Plaintext value'));
    item.addData(formatCustom.encode(Uint8List.fromList([1, 2, 3, 4])));
    item.onRegistered.addListener(() {
      print('Clipboard registered');
    });
    item.onDisposed.addListener(() {
      print('Clipboard disposed');
    });
    // window.navigator.clipboard!.write(ClipboardItem);
    // final writer = ClipboardWriter();
    // writer.write(typeHtml, '<b><i>Html</i></b> Value');
    // writer.write(typePlaintext, 'Plaintext Value');
    // .write(customKey, Uint8List.fromList([1, 2, 3, 4]));
    // final disposeListenables = await writer.commitToClipboard();
    ClipboardWriter.instance.write([item]);
    // for (final l in disposeListenables) {
    //   l.addListener(() {
    //     print('Clipboard disposed');
    //   });
    // }
  }

  void copyLazy() async {
    final item = ClipboardWriterItem();
    item.addData(formatHtml.encodeLazy(() => 'Lazy <b><i>Html</i></b> Value'));
    item.addData(formatPlainText.encodeLazy(() => 'Lazy Plaintext value'));
    item.addData(
        formatCustom.encodeLazy(() => Uint8List.fromList([1, 2, 3, 4])));
    item.onRegistered.addListener(() {
      print('Clipboard lazy registered');
    });
    item.onDisposed.addListener(() {
      print('Clipboard lazy disposed');
    });
    ClipboardWriter.instance.write([item]);

    // final writer = ClipboardWriter();
    // writer.writeLazy(typeHtml, () async {
    // await Future.delayed(Duration(seconds: 2));
    // print('Producing lazy plain text value');
    // return '<b>Lazy <i>Html</i></b> Value';
    // });
    // writer.writeLazy(typePlaintext, () {
    // print('Producing lazy html value');
    // return 'Lazy Plaintext Value';
    // });
    // writer.writeLazy(customKey, () {
    //   // print('Producing lazy custom value');
    //   return Uint8List.fromList([1, 2, 3, 4, 5]);
    // });
    // final disposeListenables = await writer.commitToClipboard();
    // for (final l in disposeListenables) {
    //   l.addListener(() {
    //     print('Clipboard lazy disposed');
    //   });
    // }
  }

  void paste() async {
    final reader = await ClipboardReader.readClipboard();
    final plainText = await reader.readValue(formatPlainText);
    final html = await reader.readValue(formatHtml);
    final custom = await reader.readValue(formatCustom);
    setState(() {
      _content =
          "Clipboard content:\n\nplaintext: $plainText\n\nhtml: $html\n\ncustom: $custom";
    });
  }

  void startDrag(BuildContext context, Offset globalPosition) async {
    // final rect = MatrixUtils.transformRect(renderObject.getTransformTo(null),
    //     Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));

    final data = raw.DataProvider(

        // DataSourceItem(representations: [
        //   DataSourceItemRepresentation.simple(
        //       formats: ['public.file-url'],
        //       data: utf8.encode('file:///tmp/test.txt')),
        //   DataSourceItemRepresentation.virtualFile(
        //       format: 'public.utf8-plain-text',
        //       storageSuggestion: VirtualFileStorage.temporaryFile,
        //       virtualFileProvider: (sinkProvider, progress) {
        //         final sink = sinkProvider(fileSize: 41);
        //         print('Writing and close');
        //         // sink.add(utf8.encode('Hello World 1\n'));
        //         // Future.delayed(const Duration(seconds: 3), () {
        //         //   sink.add(utf8.encode('Hello World 2!'));
        //         //   sink.close();
        //         // });
        //         final cancelled = [false];
        //         print('Requested file');
        //         progress.onCancel.addListener(() {
        //           print('Cancelled');
        //           cancelled[0] = true;
        //         });
        //         for (var i = 0; i < 10; ++i) {
        //           Future.delayed(Duration(milliseconds: i * 1000), () {
        //             if (cancelled[0]) {
        //               return;
        //             }
        //             progress.updateProgress(i * 10);
        //             if (i == 9) {
        //               print('Done');
        //               sink.add(utf8.encode('Hello, cruel world!\n'));
        //               sink.add(utf8.encode('Hello, cruel world!'));
        //               // sink.addError('Something went wrong');
        //               sink.close();
        //             }
        //           });
        //         }
        //       }),
        // ], suggestedName: 'File1.txt'),
        representations: [
          // DataSourceItemRepresentation.simple(
          //     formats: ['public.file-url'],
          //     data: utf8.encode('file:///tmp/test.txt')),
          raw.DataRepresentation.lazy(
              format: 'public.utf8-plain-text2',
              dataProvider: () => 'plain text lazy'),
          // DataRepresentation.lazy(
          //     // format: 'public.utf8-plain-text',
          //     format: 'text/uri-list',
          //     // format: 'text/plain',
          //     dataProvider: () => utf8.encode('baaad')),

          raw.DataRepresentation.virtualFile(
              format: 'public.utf8-plain-text',
              // format: 'text/plain',
              storageSuggestion: raw.VirtualFileStorage.temporaryFile,
              virtualFileProvider: (sinkProvider, progress) async {
                final line = utf8.encode("This is a single line\n");
                const count = 100;
                final sink = sinkProvider(fileSize: line.length * count);
                print('Writing and close');
                // sink.add(utf8.encode('Hello World 1\n'));
                // Future.delayed(const Duration(seconds: 3), () {
                //   sink.add(utf8.encode('Hello World 2!'));
                //   sink.close();
                // });
                final cancelled = [false];
                print('Requested file');
                progress.onCancel.addListener(() {
                  print('Cancelled');
                  cancelled[0] = true;
                });
                for (var i = 0; i < count; ++i) {
                  if (cancelled[0]) {
                    return;
                  }
                  sink.add(line);
                  progress.updateProgress(i.toDouble() / count.toDouble());
                  await Future.delayed(const Duration(milliseconds: 100));
                }
                // sink.close();
                // sink.addError("Something bad");
                sink.close();
                // for (var i = 0; i < 10; ++i) {
                //   Future.delayed(Duration(milliseconds: i * 1000), () {
                //     if (cancelled[0]) {
                //       return;
                //     }
                //     progress.updateProgress(i * 10);
                //     if (i == 9) {
                //       print('Done');
                //       sink.add(utf8.encode('Hello, cruel world!\n'));
                //       sink.add(utf8.encode('Hello, cruel world!'));
                //       // sink.addError('Something went wrong');
                //       sink.close();
                //     }
                //   });
                // }
              }),
        ], suggestedName: 'File2.txt');
    final handle = await data.register();

    final dragContext = await DragContext.instance();
    final session = dragContext.newSession();
    await dragContext.startDrag(
      session: session,
      configuration: DragConfiguration(allowedOperations: [
        DropOperation.copy,
        DropOperation.move,
        DropOperation.link
      ], items: [
        DragItem(
          localData: {
            'x': 1,
            'y': 2,
          },
          dataProvider: handle,
          image: await dragContainer.currentState!
              .getDragImageForOffset(globalPosition),
        )
      ]),
      position: globalPosition,
    );
    session.dragCompleted.addListener(() {
      print('Drag completed ${session.dragCompleted.value}');
    });
    // session.sessionIsDoneWithDataSource.addListener(() {
    //   print('Done with source');
    // });
    session.lastScreenLocation.addListener(() {
      // print('Last screen location ${session.lastScreenLocation.value}');
    });
  }

  String _content = "";

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: Center(
        child: Column(
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const Text(
              'You have pushed the button this many times:',
            ),
            TextButton(onPressed: copy, child: const Text('Copy')),
            TextButton(onPressed: copyLazy, child: const Text('Copy Lazy')),
            TextButton(onPressed: paste, child: const Text('Paste')),
            Text(_content),
            RepaintBoundary(
              child: DragContainer(
                key: dragContainer,
                child: GestureDetector(
                  child: Container(
                    decoration: BoxDecoration(
                        border: Border.all(color: Colors.red),
                        color: const Color.fromARGB(255, 255, 0, 0)),
                    padding: const EdgeInsets.all(10),
                    child: const Text('Drag me'),
                  ),
                  onPanStart: (details) {
                    print('Start drag');
                    startDrag(context, details.globalPosition);
                  },
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
