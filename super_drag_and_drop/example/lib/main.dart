import 'dart:convert';
import 'dart:ui' as ui;

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_clipboard_example/widget_for_reader.dart';

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
        snackBarTheme: const SnackBarThemeData(
          behavior: SnackBarBehavior.floating,
        ),
        primarySwatch: Colors.blue,
      ),
      home: const MyHomePage(title: 'SuperDrag&Drop Example'),
    );
  }
}

class MyHomePage extends StatefulWidget {
  const MyHomePage({super.key, required this.title});

  final String title;

  @override
  State<MyHomePage> createState() => _MyHomePageState();
}

class DragableWidget extends StatefulWidget {
  const DragableWidget({
    super.key,
    required this.name,
    required this.color,
    required this.dragItemProvider,
  });

  final String name;
  final Color color;
  final DragItemProvider dragItemProvider;

  @override
  State<DragableWidget> createState() => _DragableWidgetState();
}

class _DragableWidgetState extends State<DragableWidget> {
  bool _dragging = false;

  Future<DragItem?> provideDragItem(
      AsyncValueGetter<DragImage> snapshot, DragSession session) async {
    final item = await widget.dragItemProvider(snapshot, session);
    if (item != null) {
      setState(() {
        _dragging = session.dragging;
      });
      session.dragStarted.addListener(() {
        setState(() {
          _dragging = true;
        });
      });
      session.dragCompleted.addListener(() {
        if (mounted) {
          setState(() {
            _dragging = false;
          });
        }
      });
    }
    return item;
  }

  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      canAddItemToExistingSession: true,
      dragItemProvider: provideDragItem,
      child: DraggableWidget(
        child: AnimatedOpacity(
          opacity: _dragging ? 0.5 : 1,
          duration: const Duration(milliseconds: 200),
          child: Container(
            decoration: BoxDecoration(
              color: widget.color,
              borderRadius: BorderRadius.circular(14),
            ),
            padding: const EdgeInsets.symmetric(horizontal: 20, vertical: 14),
            child: Text(
              widget.name,
              style: const TextStyle(fontSize: 20, color: Colors.white),
              textAlign: TextAlign.center,
            ),
          ),
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

class HomeLayout extends StatelessWidget {
  const HomeLayout({
    super.key,
    required this.draggable,
    required this.dropZone,
  });

  final List<Widget> draggable;
  final Widget dropZone;

  @override
  Widget build(BuildContext context) {
    return SafeArea(
      child: LayoutBuilder(builder: (context, constraints) {
        if (constraints.maxWidth < 500) {
          return Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              Container(
                padding: const EdgeInsets.all(16),
                child: Wrap(
                  direction: Axis.horizontal,
                  runSpacing: 8,
                  spacing: 10,
                  children: draggable,
                ),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(16.0).copyWith(top: 0),
                  child: dropZone,
                ),
              ),
            ],
          );
        } else {
          return Row(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            textDirection: TextDirection.rtl,
            children: [
              SingleChildScrollView(
                padding: const EdgeInsets.all(16),
                child: IntrinsicWidth(
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.stretch,
                    children: draggable
                        .intersperse(
                          const SizedBox(height: 10),
                        )
                        .toList(growable: false),
                  ),
                ),
              ),
              Expanded(
                child: Padding(
                  padding: const EdgeInsets.all(16.0).copyWith(right: 0),
                  child: dropZone,
                ),
              ),
            ],
          );
        }
      }),
    );
  }
}

extension on DragSession {
  Future<bool> hasLocalData(Object data) async {
    final localData = await getLocalData() ?? [];
    return localData.contains(data);
  }
}

class _MyHomePageState extends State<MyHomePage> {
  void showMessage(String message) {
    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(message),
        duration: const Duration(milliseconds: 1500),
      ),
    );
  }

  Future<DragItem?> textDragItem(
    AsyncValueGetter<DragImage> dragImage,
    DragSession session,
  ) async {
    // For multi drag on iOS check if this item is already in the session
    if (await session.hasLocalData('text-item')) {
      return null;
    }
    final item = DragItem(
        image: await dragImage(),
        localData: 'text-item',
        suggestedName: 'PlainText.txt');
    item.add(Formats.plainText('Plain Text Value'));
    return item;
  }

  Future<DragItem?> imageDragItem(
    AsyncValueGetter<DragImage> dragImage,
    DragSession session,
  ) async {
    // For multi drag on iOS check if this item is already in the session
    if (await session.hasLocalData('image-item')) {
      return null;
    }
    final item = DragItem(
      image: await dragImage(),
      localData: 'image-item',
      suggestedName: 'Green.png',
    );
    item.add(Formats.png(await createImageData(Colors.green)));
    return item;
  }

  Future<DragItem?> lazyImageDragItem(
    AsyncValueGetter<DragImage> dragImage,
    DragSession session,
  ) async {
    // For multi drag on iOS check if this item is already in the session
    if (await session.hasLocalData('lazy-image-item')) {
      return null;
    }
    final item = DragItem(
      image: await dragImage(),
      localData: 'lazy-image-item',
      suggestedName: 'LazyBlue.png',
    );
    item.add(Formats.png.lazy(() async {
      showMessage('Requested lazy image.');
      return await createImageData(Colors.blue);
    }));
    return item;
  }

  Future<DragItem?> virtualFileDragItem(
    AsyncValueGetter<DragImage> dragImage,
    DragSession session,
  ) async {
    // For multi drag on iOS check if this item is already in the session
    if (await session.hasLocalData('virtual-file-item')) {
      return null;
    }
    final item = DragItem(
      image: await dragImage(),
      localData: 'virtual-file-item',
      suggestedName: 'VirtualFile.txt',
    );
    if (!item.virtualFileSupported) {
      return null;
    }
    item.addVirtualFile(
      format: Formats.plainText,
      provider: (sinkProvider, progress) {
        showMessage('Requesting virtual file content.');
        final line = utf8.encode('Line in virtual file\n');
        const lines = 10;
        final sink = sinkProvider(fileSize: line.length * lines);
        for (var i = 0; i < lines; ++i) {
          sink.add(line);
        }
        sink.close();
      },
    );
    return item;
  }

  Future<DragItem?> multipleRepresentationsDragItem(
    AsyncValueGetter<DragImage> dragImage,
    DragSession session,
  ) async {
    // For multi drag on iOS check if this item is already in the session
    if (await session.hasLocalData('multiple-representations-item')) {
      return null;
    }
    final item = DragItem(
      image: await dragImage(),
      localData: 'multiple-representations-item',
    );
    item.add(Formats.png(await createImageData(Colors.pink)));
    item.add(Formats.plainText("Hello World"));
    item.add(Formats.uri(
        NamedUri(Uri.parse('https://flutter.dev'), name: 'Flutter')));
    return item;
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(widget.title),
      ),
      body: HomeLayout(
        draggable: [
          DragableWidget(
            name: 'Text',
            color: Colors.red,
            dragItemProvider: textDragItem,
          ),
          DragableWidget(
            name: 'Image',
            color: Colors.green,
            dragItemProvider: imageDragItem,
          ),
          DragableWidget(
            name: 'Image 2',
            color: Colors.blue,
            dragItemProvider: lazyImageDragItem,
          ),
          DragableWidget(
            name: 'Virtual',
            color: Colors.amber.shade700,
            dragItemProvider: virtualFileDragItem,
          ),
          DragableWidget(
            name: 'Multiple',
            color: Colors.pink,
            dragItemProvider: multipleRepresentationsDragItem,
          ),
        ],
        dropZone: Container(
          decoration: BoxDecoration(
            border: Border.all(color: Colors.blueGrey.shade200),
            borderRadius: BorderRadius.circular(14),
          ),
          child: _DropZone(),
        ),
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
        ...Formats.standardFormats,
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
          borderRadius: BorderRadius.circular(13),
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
  Widget _content = const Center(
    child: Text(
      'Drop here',
      style: TextStyle(
        color: Colors.grey,
        fontSize: 16,
      ),
    ),
  );
}

class _DropItemInfo extends StatelessWidget {
  const _DropItemInfo({
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
