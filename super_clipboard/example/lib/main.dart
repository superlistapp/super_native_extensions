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

void main() async {
  final context = await RawDragDropContext.instance();
  await context.registerDropTypes([
    'public.file-url',
    'NSFilenamesPboardType',
    'public.url',
    'Apple URL pasteboard type',
  ]);
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
    final disposeListener = await writer.commitToClipboard();
    disposeListener.addListener(() {
      print('Clipboard disposed');
    });
  }

  void copyLazy() async {
    final writer = ClipboardWriter();
    writer.writeLazy(typeHtml, () {
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
    final disposeListener = await writer.commitToClipboard();
    disposeListener.addListener(() {
      print('Clipboard lazy disposed');
    });
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
    final renderObject_ = context.findRenderObject();
    final renderObject = renderObject_ is RenderRepaintBoundary
        ? renderObject_
        : context.findAncestorRenderObjectOfType<RenderRepaintBoundary>();
    final pr = MediaQuery.of(context).devicePixelRatio;
    if (renderObject == null) {
      throw DragException("Couldn't find any repaint boundary ancestor");
    }
    final snapshot = await renderObject.toImage(pixelRatio: pr);
    // final rect = MatrixUtils.transformRect(renderObject.getTransformTo(null),
    //     Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height));
    final transform = renderObject.getTransformTo(null);
    transform.invert();
    final point = MatrixUtils.transformPoint(transform, globalPosition);

    // final data = RawClipboardWriterData([
    //   RawClipboardWriterItem([
    //     RawClipboardWriterItemData.simple(
    //         types: ['public.file-url'],
    //         data: utf8.encode('file:///tmp/test.txt')),
    //   ]),
    // ]);
    // final writer = await RawClipboardWriter.withData(data);

    // final dragContext = await RawDragDropContext.instance();
    // await dragContext.startDrag(
    //   request: DragRequest(
    //     image: snapshot,
    //     writer: writer,
    //     pointInRect: point,
    //   ),
    // );
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
              child: Builder(builder: (context) {
                return GestureDetector(
                  child: Container(
                    decoration:
                        BoxDecoration(border: Border.all(color: Colors.red)),
                    padding: const EdgeInsets.all(10),
                    child: const Text('Drag me'),
                  ),
                  onPanStart: (details) {
                    print('Start drag');
                    startDrag(context, details.globalPosition);
                  },
                );
              }),
            ),
          ],
        ),
      ),
    );
  }
}
