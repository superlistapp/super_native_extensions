import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'mutex.dart';
import 'util.dart';
import 'raw_clipboard_writer.dart';

const _flutterChannel = MethodChannel('super_native_extensions');

final _channel = NativeMethodChannel('DragDropManager',
    context: superNativeExtensionsContext);

class RawDragDropContext {
  RawDragDropContext._();

  static RawDragDropContext? _instance;
  static final _mutex = Mutex();

  Future<void> _initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    final view = await _flutterChannel.invokeMethod('getFlutterView');
    await _channel.invokeMethod("newContext", {'viewHandle': view});
  }

  static Future<RawDragDropContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDragDropContext._();
        await _instance!._initialize();
      }
      return _instance!;
    });
  }

  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }

  Future<void> startDrag({
    required RawClipboardWriter writer,
    required Rect rect,
  }) {
    return _channel.invokeMethod(
        "startDrag", {'writer_id': writer.handle, 'rect': rect.serialize()});
  }
}
