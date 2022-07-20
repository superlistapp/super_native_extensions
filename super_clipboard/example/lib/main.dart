import 'dart:convert';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart';
import 'package:super_native_extensions/raw_clipboard.dart';

class DragException implements Exception {
  final String message;
  DragException(this.message);
}

class _DropDelegate implements RawDropContextDelegate {
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
    print('Drop update $event');
    return DropOperation.move;
  }

  @override
  Future<void> onPerformDrop(DropEvent event) async {
    final reader = event.reader!;
    final items = await reader.getItems();
    print('Item count ${items.length}');
    final item = items[0];
    // final data = await
    item.getDataForFormat('public.url').then((data) {
      print('URL: ${data}');
    });
    // print('DATA $data');
    print('Perform drop $event');
    // Future.delayed(Duration(seconds: 2), () {
    reader.dispose();
    // });
  }
}

class _DragDelegate implements RawDragContextDelegate {
  @override
  Future<DragConfiguration?> getConfigurationForDragRequest(
      {required Offset location, required DragSession session}) async {
    session.dragCompleted.addListener(() {
      print("Drag completed ${session.dragCompleted.value}");
    });
    session.sessionIsDoneWithDataSource.addListener(() {
      print("Session is done with data source");
    });
    session.lastScreenLocation.addListener(() {
      print('Last screen location ${session.lastScreenLocation.value}');
    });
    final data = DataProvider(suggestedName: "File1.txt", representations: [
      DataRepresentation.virtualFile(
          format: 'public.utf8-plain-text',
          storageSuggestion: VirtualFileStorage.temporaryFile,
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
                progress.updateProgress(i * 10);
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
}

final dragContainer = GlobalKey<DragContainerState>();

void main() async {
  // final dropContext = await RawDropContext.instance();
  // await dropContext.registerDropTypes([
  //   'public.file-url',
  //   'NSFilenamesPboardType',
  //   'public.url',
  //   'Apple URL pasteboard type',
  // ]);
  await RawDragContext.instance();
  await RawDropContext.instance();
  (await RawDragContext.instance()).delegate = _DragDelegate();
  (await RawDropContext.instance()).delegate = _DropDelegate();
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

final customKey = CustomClipboardType<Uint8List>(
    "com.superlist.clipboard.Example.CustomType");

class _MyHomePageState extends State<MyHomePage> {
  void copy() async {
    final writer = ClipboardWriter();
    writer.write(typeHtml, '<b><i>Html</i></b> Value');
    writer.write(typePlaintext, 'Plaintext Value');
    writer.write(customKey, Uint8List.fromList([1, 2, 3, 4]));
    final disposeListenables = await writer.commitToClipboard();
    for (final l in disposeListenables) {
      l.addListener(() {
        print('Clipboard disposed');
      });
    }
  }

  void copyLazy() async {
    final writer = ClipboardWriter();
    writer.writeLazy(typeHtml, () async {
      // await Future.delayed(Duration(seconds: 2));
      // print('Producing lazy plain text value');
      return '<b>Lazy <i>Html</i></b> Value';
    });
    writer.writeLazy(typePlaintext, () {
      // print('Producing lazy html value');
      return 'Lazy Plaintext Value';
    });
    writer.writeLazy(customKey, () {
      // print('Producing lazy custom value');
      return Uint8List.fromList([1, 2, 3, 4, 5]);
    });
    final disposeListenables = await writer.commitToClipboard();
    for (final l in disposeListenables) {
      l.addListener(() {
        print('Clipboard lazy disposed');
      });
    }
  }

  void paste() async {
    final reader = await ClipboardReader.newDefaultReader();
    final plainText = await reader.readValue(typePlaintext);
    final html = await reader.readValue(typeHtml);
    final custom = await reader.readValue(customKey);
    setState(() {
      _content =
          "Clipboard content:\n\nplaintext: $plainText\n\nhtml: $html\n\ncustom: $custom";
    });
  }

  void startDrag(BuildContext context, Offset globalPosition) async {
    // final rect = MatrixUtils.transformRect(renderObject.getTransformTo(null),
    //     Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));

    final data = DataProvider(

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
          // DataRepresentation.lazy(
          //     // formats: ['public.utf8-plain-text'], data: utf8.encode('baaad')),
          //     format: 'text/plain',
          //     dataProvider: () => utf8.encode('baaad')),
          DataRepresentation.virtualFile(
              format: 'public.utf8-plain-text',
              // format: 'text/plain',
              storageSuggestion: VirtualFileStorage.temporaryFile,
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
                  progress.updateProgress(i);
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

    final dragContext = await RawDragContext.instance();
    final session = await dragContext.startDrag(
      request: DragRequest(
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
      ),
    );
    session.dragCompleted.addListener(() {
      print('Drag completed ${session.dragCompleted.value}');
    });
    session.sessionIsDoneWithDataSource.addListener(() {
      print('Done with source');
    });
    session.lastScreenLocation.addListener(() {
      print('Last screen location ${session.lastScreenLocation.value}');
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
