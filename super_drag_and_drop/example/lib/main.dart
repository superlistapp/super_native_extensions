import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:collection/collection.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

import 'widget_for_reader.dart';

FutureOr<void> x() {
  // return SynchronousFuture(null);
}

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
      home: const MyHomePage(title: 'Flutter Demo Home Page'),
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
    required this.fileName,
    required this.payload,
    required this.localData,
  });

  final String name;
  final Color color;
  final String fileName;
  final String payload;
  final Object localData;

  @override
  Widget build(BuildContext context) {
    return DragItemWidget(
      allowedOperations: () => [DropOperation.copy],
      canAddItemToExistingSession: true,
      dragItem: (snapshot, session) async {
        final sessionLocalData = await session.getLocalData() ?? [];
        if (sessionLocalData.contains(localData)) {
          return null;
        }
        session.dragCompleted.addListener(() {
          print('Drag res ${session.dragCompleted.value}');
        });
        final item = DragItem(
          image: await snapshot(),
          localData: localData,
          suggestedName: fileName,
        );
        // item.addData(formatPlainText.encode(payload));
        return item;
      },
      child: DraggableWidget(
        child: Container(
          decoration: BoxDecoration(
            color: color,
            // border: Border.all(
            //   color: Colors.black,
            // )
          ),
          padding: const EdgeInsets.all(20),
          alignment: Alignment.center,
          child: Text(name, style: const TextStyle(fontSize: 25)),
        ),
      ),
    );
  }
}

const jpeg = SimpleDataFormat<Uint8List>(
  fallback: SimplePlatformCodec(formats: ['image/jpeg']),
);

const uriList = SimpleDataFormat<Uint8List>(
  fallback: SimplePlatformCodec(formats: ['uriList']),
);

const fileUrl = SimpleDataFormat<Uint8List>(
  fallback: SimplePlatformCodec(formats: ['public.file-url']),
);

class _MyHomePageState extends State<MyHomePage> {
  final item1 = GlobalKey<DragItemWidgetState>();
  final item2 = GlobalKey<DragItemWidgetState>();

  var contents = <Widget>[];

  @override
  Widget build(BuildContext context) {
    // This method is rerun every time setState is called, for instance as done
    // by the _incrementCounter method above.
    //
    // The Flutter framework has been optimized to make rerunning build methods
    // fast, so that you can just rebuild anything that needs updating rather
    // than having to individually change instances of widgets.
    return Scaffold(
      appBar: AppBar(
        // Here we take the value from the MyHomePage object that was created by
        // the App.build method, and use it to set our appbar title.
        title: Text(widget.title),
      ),
      body: Center(
        // Center is a layout widget. It takes a single child and positions it
        // in the middle of the parent.
        child: Column(
          // Column is also a layout widget. It takes a list of children and
          // arranges them vertically. By default, it sizes itself to fit its
          // children horizontally, and tries to be as tall as its parent.
          //
          // Invoke "debug painting" (press "p" in the console, choose the
          // "Toggle Debug Paint" action from the Flutter Inspector in Android
          // Studio, or the "Toggle Debug Paint" command in Visual Studio Code)
          // to see the wireframe for each widget.
          //
          // Column has various properties to control how it sizes itself and
          // how it positions its children. Here we use mainAxisAlignment to
          // center the children vertically; the main axis here is the vertical
          // axis because Columns are vertical (the cross axis would be
          // horizontal).
          mainAxisAlignment: MainAxisAlignment.center,
          children: <Widget>[
            const SizedBox(
              height: 30,
            ),
            // DemoWidget(
            //   name: 'Widget 1',
            //   color: Colors.red,
            //   payload: 'Payload 1',
            //   localData: 'D1',
            //   fileName: 'File1.txt',
            // ),

            // Padding(
            //   padding: const EdgeInsets.all(40.0),
            //   child: ListView(
            //     shrinkWrap: true,
            //     children: const [
            //       DemoWidget(
            //         name: 'Widget 1',
            //         color: Colors.red,
            //         payload: 'Payload 1',
            //         localData: 'D1',
            //         fileName: 'File1.txt',
            //       ),
            //       DemoWidget(
            //         name: 'Widget 2',
            //         color: Colors.yellow,
            //         payload: 'Payload 1',
            //         localData: 'D2',
            //         fileName: 'File2.txt',
            //       ),
            //       DemoWidget(
            //         name: 'Widget 3',
            //         color: Colors.green,
            //         payload: 'Payload 3',
            //         localData: 'D3',
            //         fileName: 'File3.txt',
            //       ),
            //       DemoWidget(
            //         name: 'Widget 4',
            //         color: Colors.blue,
            //         payload: 'Payload 4',
            //         localData: 'D4',
            //         fileName: 'File4.txt',
            //       ),
            //     ],
            //   ),
            // ),
            DropRegion(
              formats: const [
                ...Format.standardFormats,
                formatCustom,
              ],
              onDropOver: (session, position) async {
                // print('OnDropOver $position, $session');
                return DropOperation.copy;
              },
              onDropLeave: (session) async {
                print('Drop leave');
              },
              onPerformDrop: (session, position, acceptedOperation) async {
                print('Perform drop $acceptedOperation');

                print(
                    'session ${session.toString(minLevel: DiagnosticLevel.fine)}');
                // for (final item in session.items) {
                // print('Item ${item.toString()}');
                // final readerItem = item.readerItem;
                // print('Formats: ${item.formats}');
                // final j = await item.dataReader!.readValue(jpeg);
                // final j = await item.dataReader!.readValue(uriList);
                // final name = await item.dataReader!.suggestedName();
                // print('__ $name');
                // print('J $j');
                // if (readerItem != null) {
                // print('Drop update ${await readerItem.getAvailableFormats()}');
                // }
                // }
                for (final item in session.items) {
                  final reader = item.dataReader!;
                  final receiver = await reader.getVirtualFileReceiver(
                      format: Format.imageJpeg);
                  // if (receiver != null) {
                  //   print('FORMAT ${receiver.format}');
                  //   receiver
                  //       .receiveVirtualFile(targetFolder: '/Users/Matej/_temp')
                  //       .first
                  //       .then((value) => print('REceived file at $value'),
                  //           onError: (e) {
                  //     print('Error $e');
                  //   });
                  //   print(
                  //       'object ${await item.dataReader!.rawReader!.getSuggestedName()}');
                  // }
                }

                final readers = await Future.wait(session.items
                    .map((e) => ReaderInfo.fromReader(e.dataReader!)));

                final widgets = Future.wait(
                  readers.mapIndexed(
                    (index, element) =>
                        buildWidgetForReader(context, element, index),
                  ),
                );
                print('Built widgets');
                widgets.then((value) {
                  print('Have $value');
                  setState(() {
                    contents = value;
                  });
                }, onError: (e) {
                  print('E $e');
                });
              },
              onDropEnded: (session) {
                print('Drop ended');
              },
              child: Container(
                width: 400,
                height: 400,
                color: Colors.amber,
                child: SelectionArea(
                  focusNode: FocusNode()..canRequestFocus = false,
                  child: ListView(
                    padding: const EdgeInsets.all(16),
                    children: contents
                        .intersperse(const SizedBox(
                          height: 10,
                        ))
                        .toList(growable: false),
                  ),
                ),
              ),
            )
          ],
        ),
      ),
    );
  }
}
