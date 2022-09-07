import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_keyboard_layout/super_keyboard_layout.dart';

late KeyboardLayoutManager _keyboardLayoutManager;

void main() async {
  _keyboardLayoutManager = await KeyboardLayoutManager.instance();
  print('Supported: ${_keyboardLayoutManager.supported}');
  _keyboardLayoutManager.onLayoutChanged.addListener(() {
    print('Layout changed');
  });
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
      home: const MyHomePage(title: 'Super Keyboard Layout Home Page'),
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

class _MyHomePageState extends State<MyHomePage> {
  @override
  void initState() {
    super.initState();
    _keyboardLayoutManager.onLayoutChanged.addListener(_layoutChanged);
  }

  @override
  void dispose() {
    super.dispose();
    _keyboardLayoutManager.onLayoutChanged.removeListener(_layoutChanged);
  }

  void _layoutChanged() {
    setState(() {});
  }

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
            if (!_keyboardLayoutManager.supported)
              const Text(
                'KeyboardLayoutManager is not supported on this platform.',
              ),
            if (_keyboardLayoutManager.supported)
              Text(
                  'Physical key Y on your keyboard will result inֿ\n${_keyboardLayoutManager.currentLayout.getLogicalKeyForPhysicalKey(PhysicalKeyboardKey.bracketLeft)}'),
          ],
        ),
      ),
    );
  }
}
