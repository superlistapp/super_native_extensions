import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:super_keyboard_layout/super_keyboard_layout.dart';

late KeyboardLayoutManager _keyboardLayoutManager;

void main() async {
  _keyboardLayoutManager = await KeyboardLayoutManager.instance();
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

class LayoutDemoWidget extends StatefulWidget {
  const LayoutDemoWidget({super.key});

  @override
  State<StatefulWidget> createState() => _LayoutDemoWidgetState();
}

class _LayoutDemoWidgetState extends State<LayoutDemoWidget> {
  late FocusNode _focusNode;

  @override
  void initState() {
    super.initState();
    _focusNode = FocusNode();
    _focusNode.onKey = _onKey;
    _focusNode.requestFocus();
    _keyboardLayoutManager.onLayoutChanged.addListener(_layoutChanged);
  }

  KeyEventResult _onKey(FocusNode node, RawKeyEvent event) {
    if (event is RawKeyDownEvent) {
      // print('Key down: ${event.logicalKey}');
      setState(() {
        _lastKey = event.physicalKey;
      });
    }
    return KeyEventResult.handled;
  }

  @override
  void dispose() {
    _focusNode.dispose();
    _keyboardLayoutManager.onLayoutChanged.removeListener(_layoutChanged);
    super.dispose();
  }

  PhysicalKeyboardKey? _lastKey;

  void _layoutChanged() {
    setState(() {});
    ScaffoldMessenger.of(context).showSnackBar(
      const SnackBar(
        content: Text("Keyboard layout has changed."),
        duration: Duration(milliseconds: 1500),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.all(10),
      decoration: BoxDecoration(
        border: Border.all(color: Colors.red, width: 2),
      ),
      child: Focus(
        focusNode: _focusNode,
        child: _lastKey == null
            ? const Text('Press any key')
            : Column(
                crossAxisAlignment: CrossAxisAlignment.start,
                children: [
                  const Text("Pressed physical key:",
                      style: TextStyle(fontWeight: FontWeight.bold)),
                  Text(_lastKey!.toString()),
                  const SizedBox(
                    height: 10,
                  ),
                  const Text('Logical key for current keyboard layout:',
                      style: TextStyle(fontWeight: FontWeight.bold)),
                  Text(_keyboardLayoutManager.currentLayout
                          .getLogicalKeyForPhysicalKey(_lastKey!)
                          ?.toString() ??
                      'null'),
                  const Text(
                      'Logical key for current keyboard layout (with shift):',
                      style: TextStyle(fontWeight: FontWeight.bold)),
                  Text(_keyboardLayoutManager.currentLayout
                          .getLogicalKeyForPhysicalKey(_lastKey!, shift: true)
                          ?.toString() ??
                      'null'),
                  const Text(
                      'Logical key for current keyboard layout (with alt):',
                      style: TextStyle(fontWeight: FontWeight.bold)),
                  Text(_keyboardLayoutManager.currentLayout
                          .getLogicalKeyForPhysicalKey(_lastKey!, alt: true)
                          ?.toString() ??
                      'null'),
                  const SizedBox(
                    height: 20,
                  ),
                  const Text(
                    'Because this functionality is meant for '
                    'keyboard shortcuts, only ASCII capable keyboard '
                    'layouts are supported.\nOn Linux switching between '
                    'keyboards is only recognized after typing a key from'
                    ' new layout.',
                    style: TextStyle(fontSize: 11.5),
                  ),
                ],
              ),
      ),
    );
  }
}

class MyHomePage extends StatelessWidget {
  const MyHomePage({super.key, required this.title});

  final String title;

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: Text(title),
      ),
      body: Padding(
        padding: const EdgeInsets.all(20.0),
        child: Center(
          child: Column(
            mainAxisAlignment: MainAxisAlignment.center,
            children: <Widget>[
              if (!_keyboardLayoutManager.supported)
                const Text(
                  'KeyboardLayoutManager is not supported on this platform.',
                ),
              if (_keyboardLayoutManager.supported) const LayoutDemoWidget(),
            ],
          ),
        ),
      ),
    );
  }
}
