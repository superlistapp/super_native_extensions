import '../keyboard_layout.dart';
import '../util.dart';

class KeyboardLayoutManagerImpl extends KeyboardLayoutManager {
  static final _instance = KeyboardLayoutManagerImpl();

  static Future<KeyboardLayoutManager> instance() => Future.value(_instance);

  static final _currentLayout = KeyboardLayout({}, {}, {});

  @override
  KeyboardLayout get currentLayout => _currentLayout;

  @override
  final onLayoutChanged = SimpleNotifier();

  @override
  bool get supported => false;
}
