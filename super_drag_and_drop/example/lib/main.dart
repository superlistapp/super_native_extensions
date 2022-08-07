import 'dart:async';
import 'dart:typed_data';

import 'package:flutter/material.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';

FutureOr<void> x() {
  // return SynchronousFuture(null);
}

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

final jpeg =
    SimpleDataFormat<Uint8List>.passthrough(defaultFormat: 'image/jpeg');

final uriList =
    SimpleDataFormat<String>.passthrough(defaultFormat: 'text/uri-list');

class _MyHomePageState extends State<MyHomePage> {
  int _counter = 0;

  void _incrementCounter() {
    setState(() {
      // This call to setState tells the Flutter framework that something has
      // changed in this State, which causes it to rerun the build method below
      // so that the display can reflect the updated values. If we changed
      // _counter without calling setState(), then the build method would not be
      // called again, and so nothing would appear to happen.
      _counter++;
    });
  }

  final item1 = GlobalKey<DragItemWidgetState>();
  final item2 = GlobalKey<DragItemWidgetState>();

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
            const Text(
              'You have pushed the button this many times:',
            ),
            Text(
              '$_counter',
              style: Theme.of(context).textTheme.headline4,
            ),
            const SizedBox(
              height: 30,
            ),
            Padding(
              padding: const EdgeInsets.all(40.0),
              child: ListView(
                shrinkWrap: true,
                children: const [
                  DemoWidget(
                    name: 'Widget 1',
                    color: Colors.red,
                    payload: 'Payload 1',
                    localData: 'D1',
                    fileName: 'File1.txt',
                  ),
                  DemoWidget(
                    name: 'Widget 2',
                    color: Colors.yellow,
                    payload: 'Payload 1',
                    localData: 'D2',
                    fileName: 'File2.txt',
                  ),
                  DemoWidget(
                    name: 'Widget 3',
                    color: Colors.green,
                    payload: 'Payload 3',
                    localData: 'D3',
                    fileName: 'File3.txt',
                  ),
                  DemoWidget(
                    name: 'Widget 4',
                    color: Colors.blue,
                    payload: 'Payload 4',
                    localData: 'D4',
                    fileName: 'File4.txt',
                  ),
                ],
              ),
            ),
            BaseDropRegion(
              formats: const [
                // formatPlainText,
              ],
              onDropOver: (session, position) async {
                print('OnDropOver $position');
                return DropOperation.copy;
              },
              onDropLeave: (session) async {
                print('Drop leave');
              },
              onPerformDrop: (session, position, acceptedOperation) async {
                print('Perform drop $acceptedOperation');
                for (final item in session.items) {
                  // final readerItem = item.readerItem;
                  // print('Formats: ${item.formats}');
                  // final j = await item.dataReader!.readValue(jpeg);
                  final j = await item.dataReader!.readValue(uriList);
                  print('J $j');
                  // if (readerItem != null) {
                  // print('Drop update ${await readerItem.getAvailableFormats()}');
                  // }
                }
              },
              onDropEnded: (session) {
                print('Drop ended');
              },
              child: Container(
                width: 100,
                height: 100,
                color: Colors.amber,
              ),
            )
          ],
        ),
      ),
      floatingActionButton: FloatingActionButton(
        onPressed: _incrementCounter,
        tooltip: 'Increment',
        child: const Icon(Icons.add),
      ), // This trailing comma makes auto-formatting nicer for build methods.
    );
  }
}
