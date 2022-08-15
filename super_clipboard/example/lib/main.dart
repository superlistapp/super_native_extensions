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
        outlinedButtonTheme: OutlinedButtonThemeData(
          style: OutlinedButton.styleFrom(
              padding:
                  const EdgeInsets.symmetric(horizontal: 20, vertical: 18)),
        ),
        primarySwatch: Colors.blue,
        useMaterial3: false,
      ),
      home: const MyHomePage(title: 'SuperClipboard Example'),
    );
  }
}

class TabLayout extends StatefulWidget {
  const TabLayout({
    super.key,
    required this.copy,
    required this.paste,
    required this.tabController,
  });

  @override
  State<TabLayout> createState() => _TabLayoutState();

  final TabController tabController;
  final Widget copy;
  final Widget paste;
}

class _TabLayoutState extends State<TabLayout> {
  @override
  Widget build(BuildContext context) {
    return Column(
      children: [
        Container(
          color: Colors.blueGrey.shade50,
          child: TabBar(
            controller: widget.tabController,
            labelColor: Colors.black,
            tabs: const [
              Tab(text: 'Copy'),
              Tab(text: 'Clipboard Viewer'),
            ],
          ),
        ),
        Expanded(
          child: TabBarView(
            controller: widget.tabController,
            children: [
              widget.copy,
              widget.paste,
            ],
          ),
        )
      ],
    );
  }
}

class SideBySideLayout extends StatelessWidget {
  const SideBySideLayout({
    super.key,
    required this.copy,
    required this.paste,
  });

  final Widget copy;
  final Widget paste;

  @override
  Widget build(BuildContext context) {
    return Row(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        copy,
        VerticalDivider(
          color: Colors.blueGrey.shade100,
          thickness: 1,
          width: 1,
        ),
        Expanded(child: paste),
      ],
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

class _MyHomePageState extends State<MyHomePage>
    with SingleTickerProviderStateMixin {
  final _copyKey = GlobalKey();
  final _pasteKey = GlobalKey<_PasteSectionState>();

  late final TabController _tabController;

  @override
  void initState() {
    super.initState();
    _tabController = TabController(length: 2, vsync: this);
  }

  @override
  void dispose() {
    super.dispose();
    _tabController.dispose();
  }

  void showMessage(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        duration: const Duration(milliseconds: 1500),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: Center(
        child: LayoutBuilder(builder: (context, constraints) {
          final copySection = _CopySection(
            key: _copyKey,
            onShowMessage: showMessage,
          );
          final pasteSection = _PasteSection(key: _pasteKey);
          if (constraints.maxWidth < 450) {
            return TabLayout(
              copy: copySection,
              paste: pasteSection,
              tabController: _tabController,
            );
          } else {
            _tabController.index = 1;
            return SideBySideLayout(
              copy: copySection,
              paste: pasteSection,
            );
          }
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
  const _CopySection({
    Key? key,
    required this.onShowMessage,
  }) : super(key: key);

  final void Function(String) onShowMessage;

  @override
  State<_CopySection> createState() => _CopySectionState();
}

class _CopySectionState extends State<_CopySection> {
  void copyText() async {
    final item = DataWriterItem();
    item.add(Format.htmlText.encode('<b>This is a <em>HTML</en> value</b>.'));
    item.add(Format.plainText.encode('This is a plaintext value.'));
    await ClipboardWriter.instance.write([item]);
  }

  void copyTextLazy() async {
    final showMessage = widget.onShowMessage;
    final item = DataWriterItem();
    item.add(Format.htmlText.encodeLazy(() {
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
    final showMessage = widget.onShowMessage;
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
    final showMessage = widget.onShowMessage;
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
    return Center(
      child: IntrinsicWidth(
        child: Padding(
          padding: const EdgeInsets.all(16),
          child: FocusTraversalGroup(
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.stretch,
              children: <Widget>[
                OutlinedButton(
                  onPressed: copyText,
                  child: const Text('Copy Text'),
                ),
                OutlinedButton(
                    onPressed: copyTextLazy,
                    child: const Text('Copy Text (Lazy)')),
                OutlinedButton(
                    onPressed: copyImage, child: const Text('Copy Image')),
                OutlinedButton(
                    onPressed: copyImageLazy,
                    child: const Text('Copy Image (Lazy)')),
                OutlinedButton(
                    onPressed: copyCustomData,
                    child: const Text('Copy Custom Data')),
                OutlinedButton(
                    onPressed: copyCustomDataLazy,
                    child: const Text('Copy Custom (Lazy)')),
                OutlinedButton(
                    onPressed: copyUri, child: const Text('Copy URI')),
              ].intersperse(const SizedBox(height: 10)).toList(growable: false),
            ),
          ),
        ),
      ),
    );
  }
}

class _PasteSection extends StatefulWidget {
  const _PasteSection({Key? key}) : super(key: key);

  @override
  State createState() => _PasteSectionState();
}

class _PasteSectionState extends State<_PasteSection>
    with AutomaticKeepAliveClientMixin {
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
  bool get wantKeepAlive => true;

  @override
  Widget build(BuildContext context) {
    super.build(context);
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      mainAxisSize: MainAxisSize.max,
      children: [
        Padding(
          padding: const EdgeInsets.all(16).copyWith(bottom: 0),
          child: OutlinedButton(onPressed: _paste, child: const Text('Paste')),
        ),
        Expanded(
          child: SelectionArea(
            focusNode: FocusNode()..canRequestFocus = false,
            child: ListView(
              padding: const EdgeInsets.all(16),
              children: contentWidgets,
            ),
          ),
        )
      ],
    );
  }
}
