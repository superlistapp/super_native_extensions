import 'dart:typed_data';
import 'dart:ui' as ui;

import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_clipboard_example/widget_for_reader.dart';

const formatCustom = CustomDataFormat<Uint8List>(
  applicationId: "com.superlist.clipboard.Example.CustomType",
);

void main() async {
  runApp(const MyApp());
}

class MyApp extends StatelessWidget {
  const MyApp({super.key});

  // This widget is the root of your application.
  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      title: 'Flutter Demo',
      theme: ThemeData(
        primarySwatch: Colors.blue,
      ),
      home: const MyHomePage(title: 'SuperDragAndDrop Example'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({super.key, required this.title});

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class DemoWidget extends StatelessWidget {
  const DemoWidget({
    super.key,
    required this.name,
    required this.color,
    required this.dragItemProvider,
  });

  final String name;
  final Color color;
  final DragItemProvider dragItemProvider;

  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      canAddItemToExistingSession: true,
      dragItemProvider: dragItemProvider,
      child: DraggableWidget(
        child: Container(
          decoration: BoxDecoration(
            color: color,
          ),
          padding: const EdgeInsets.all(20),
          alignment: Alignment.center,
          child: Text(name, style: const TextStyle(fontSize: 25)),
        ),
      ),
    );
  }
}

Future<Uint8List> createImageData(Color color) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  final paint = Paint()..color = color;
  canvas.drawOval(const Rect.fromLTWH(0, 0, 200, 200), paint);
  final picture = recorder.endRecording();
  final image = await picture.toImage(200, 200);
  final data = await image.toByteData(format: ui.ImageByteFormat.png);
  return data!.buffer.asUint8List();
}

class _MyHomePageState extends State<MyHomePage> {
  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: Row(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: <Widget>[
          SizedBox(
            width: 200,
            child: ListView(
              children: [
                DemoWidget(
                  name: "Red",
                  color: Colors.red,
                  dragItemProvider: (dragImage, session) async {
                    final item = DragItem(
                      image: await dragImage(),
                      localData: 10,
                    );
                    item.add(Format.imagePng
                        .encode(await createImageData(Colors.green)));
                    item.add(Format.plainText.encode("Hello World"));
                    item.add(Format.uri.encode(NamedUri(
                        Uri.parse('https://flutter.dev'),
                        name: 'Flutter')));
                    return item;
                  },
                ),
              ],
            ),
          ),
          Expanded(
            child: Padding(
              padding: const EdgeInsets.all(20),
              child: Container(
                decoration: BoxDecoration(
                  border: Border.all(color: Colors.blueGrey.shade200),
                  borderRadius: BorderRadius.circular(8),
                ),
                child: _DropZone(),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

class _DropZone extends StatefulWidget {
  @override
  State<StatefulWidget> createState() => _DropZoneState();
}

class _DropZoneState extends State<_DropZone> {
  @override
  Widget build(BuildContext context) {
    return DropRegion(
      formats: const [
        ...Format.standardFormats,
        formatCustom,
      ],
      hitTestBehavior: HitTestBehavior.opaque,
      onDropOver: _onDropOver,
      onPerformDrop: _onPerformDrop,
      onDropLeave: _onDropLeave,
      child: Stack(
        children: [
          Positioned.fill(child: _content),
          Positioned.fill(
            child: IgnorePointer(
              child: AnimatedOpacity(
                opacity: _isDragOver ? 1.0 : 0.0,
                duration: const Duration(milliseconds: 200),
                child: _preview,
              ),
            ),
          ),
        ],
      ),
    );
  }

  DropOperation _onDropOver(DropSession session, Offset _) {
    setState(() {
      _isDragOver = true;
      _preview = Container(
        decoration: BoxDecoration(
          borderRadius: BorderRadius.circular(7),
          color: Colors.black.withOpacity(0.2),
        ),
        child: Padding(
          padding: const EdgeInsets.all(50),
          child: Center(
            child: ConstrainedBox(
              constraints: const BoxConstraints(maxWidth: 400),
              child: ClipRRect(
                borderRadius: BorderRadius.circular(10),
                child: ListView(
                  shrinkWrap: true,
                  children: session.items
                      .map<Widget>((e) => _DropItemInfo(dropItem: e))
                      .intersperse(Container(
                        height: 2,
                        color: Colors.white.withOpacity(0.7),
                      ))
                      .toList(growable: false),
                ),
              ),
            ),
          ),
        ),
      );
    });
    return session.allowedOperations.firstOrNull ?? DropOperation.none;
  }

  Future<void> _onPerformDrop(
      DropSession session, Offset _, DropOperation operation) async {
    // Obtain additional reader information first
    final readers = await Future.wait(
      session.items.map(
        (e) => ReaderInfo.fromReader(
          e.dataReader!,
          localData: e.localData,
        ),
      ),
    );

    if (!mounted) {
      return;
    }

    buildWidgetsForReaders(context, readers, (value) {
      setState(() {
        _content = SelectionArea(

          focusNode: FocusNode()..canRequestFocus = false,
          child: ListView(
            padding: const EdgeInsets.all(10),
            children: value
                .intersperse(const SizedBox(height: 10))
                .toList(growable: false),
          ),
        );
      });
    });
  }

  void _onDropLeave(DropSession session) {
    setState(() {
      _isDragOver = false;
    });
  }

  bool _isDragOver = false;

  Widget _preview = const SizedBox();
  Widget _content = const SizedBox();
}

class _DropItemInfo extends StatelessWidget {
  const _DropItemInfo({
    super.key,
    required this.dropItem,
  });

  final DropItem dropItem;

  @override
  Widget build(BuildContext context) {
    return Container(
      color: Colors.white,
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 10),
      child: DefaultTextStyle.merge(
        style: const TextStyle(fontSize: 11.0),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.start,
          children: [
            if (dropItem.localData != null)
              Text.rich(TextSpan(children: [
                const TextSpan(
                  text: 'Local data: ',
                  style: TextStyle(fontWeight: FontWeight.bold),
                ),
                TextSpan(text: '${dropItem.localData}'),
              ])),
            const SizedBox(
              height: 4,
            ),
            Text.rich(TextSpan(children: [
              const TextSpan(
                text: 'Native formats: ',
                style: TextStyle(fontWeight: FontWeight.bold),
              ),
              TextSpan(text: dropItem.platformFormats.join(', ')),
            ])),
          ],
        ),
      ),
    );
  }
}
