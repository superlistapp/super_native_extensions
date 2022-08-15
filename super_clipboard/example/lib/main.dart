import 'dart:ui' as ui;
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:super_clipboard/super_clipboard.dart';

import 'widget_for_reader.dart';

void main() async {
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
        snackBarTheme: const SnackBarThemeData(
          behavior: SnackBarBehavior.floating,
        ),
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
        useMaterial3: true,
      ),
      home: const MyHomePage(title: 'SuperClipboard Example'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({Key? key, required this.title}) : super(key: key);

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

const formatCustom = CustomDataFormat<Uint8List>(
  applicationId: "com.superlist.clipboard.Example.CustomType",
);

class _MyHomePageState extends State<MyHomePage> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: Center(
        child: LayoutBuilder(builder: (context, constraints) {
          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              _CopySection(),
              Expanded(child: _PasteSection()),
            ],
          );
        }),
      ),
    );
  }
}

Future<Uint8List> createImageData(Color color) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  final paint = Paint()..color = color;
  canvas.drawOval(const Rect.fromLTWH(0, 0, 100, 100), paint);
  final picture = recorder.endRecording();
  final image = await picture.toImage(100, 100);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  return data!.buffer.asUint8List();
}

class _CopySection extends StatefulWidget {
  @override
  State<_CopySection> createState() => _CopySectionState();
}

class _CopySectionState extends State<_CopySection> {
  void showMessage(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        duration: const Duration(milliseconds: 1500),
      ),
    );
  }

  void copyText() async {
    final item = DataWriterItem();
    item.add(Format.html.encode('<b>This is a <em>HTML</en> value</b>.'));
    item.add(Format.plainText.encode('This is a plaintext value.'));
    await ClipboardWriter.instance.write([item]);
  }

  void copyTextLazy() async {
    final item = DataWriterItem();
    item.add(Format.html.encodeLazy(() {
      showMessage('Lazy rich text requested.');
      return '<b>This is a <em>HTML</en> value</b> generated <u>on demand</u>.';
    }));
    item.add(Format.plainText.encodeLazy(() {
      showMessage('Lazy plain text requested.');
      return 'This is a plaintext value generated on demand.';
    }));
    await ClipboardWriter.instance.write([item]);
  }

  void copyImage() async {
    final image = await createImageData(Colors.red);
    final item = DataWriterItem(suggestedName: 'RedCircle.png');
    item.add(Format.imagePng.encode(image));
    await ClipboardWriter.instance.write([item]);
  }

  void copyImageLazy() async {
    final item = DataWriterItem(suggestedName: 'BlueCircle.png');
    item.add(Format.imagePng.encodeLazy(() {
      showMessage('Lazy image requested.');
      return createImageData(Colors.blue);
    }));
    await ClipboardWriter.instance.write([item]);
  }

  void copyCustomData() async {
    final item = DataWriterItem();
    item.add(formatCustom.encode(Uint8List.fromList([1, 2, 3, 4])));
    await ClipboardWriter.instance.write([item]);
  }

  void copyCustomDataLazy() async {
    final item = DataWriterItem();
    item.add(formatCustom.encodeLazy(() async {
      showMessage('Lazy custom data requested.');
      return Uint8List.fromList([1, 2, 3, 4, 5, 6]);
    }));
    await ClipboardWriter.instance.write([item]);
  }

  void copyUri() async {
    final item = DataWriterItem();
    item.add(Format.uri.encode(
        NamedUri(Uri.parse('https://google.com'), name: 'Google home page')));
    await ClipboardWriter.instance.write([item]);
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        TextButton(onPressed: copyText, child: const Text('Copy Text')),
        TextButton(
            onPressed: copyTextLazy, child: const Text('Copy Text (Lazy)')),
        TextButton(onPressed: copyImage, child: const Text('Copy Image')),
        TextButton(
            onPressed: copyImageLazy, child: const Text('Copy Image (Lazy)')),
        TextButton(
            onPressed: copyCustomData, child: const Text('Copy Custom Data')),
        TextButton(
            onPressed: copyCustomDataLazy,
            child: const Text('Copy Custom (Lazy)')),
        TextButton(onPressed: copyUri, child: const Text('Copy Uri')),
      ],
    );
  }
}

class _PasteSection extends StatefulWidget {
  @override
  State createState() => _PasteSectionState();
}

class _PasteSectionState extends State<_PasteSection> {
  var contentWidgets = <Widget>[];

  void _paste() async {
    final reader = await ClipboardReader.readClipboard();

    final widgets = <Widget>[];

    int index = 0;
    for (final readerItem in reader.items) {
      if (widgets.isNotEmpty) {
        widgets.add(const SizedBox(height: 10));
      }
      widgets.add(await buildWidgetForReader(readerItem, index++));
    }

    setState(() {
      contentWidgets = widgets;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.max,
      children: [
        TextButton(onPressed: _paste, child: const Text('Paste')),
        Expanded(
          child: SelectionArea(
            child: ListView(
              padding: const EdgeInsets.all(20),
              children: contentWidgets,
            ),
          ),
        )
      ],
    );
  }
}
