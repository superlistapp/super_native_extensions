import 'dart:convert';
import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'mutex.dart';
import 'api_model.dart';
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
    _channel.setMethodCallHandler(_handleMethodCall);
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

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'writerForDragRequest') {
      print('ARG ${call.arguments}');
      final data = RawClipboardWriterData([
        RawClipboardWriterItem([
          RawClipboardWriterItemData.simple(
              types: ['public.file-url'],
              data: utf8.encode('file:///tmp/test.txt')),
        ]),
      ]);
      final writer = await RawClipboardWriter.withData(data);
      return {'writerId': writer.handle};
    } else {
      return null;
    }
  }

  Future<void> registerDropTypes(List<String> types) {
    return _channel.invokeMethod("registerDropTypes", {'types': types});
  }

  Future<void> startDrag({
    required DragRequest request,
  }) async {
    return _channel.invokeMethod("startDrag", await request.serialize());
  }
}

class DragRequest {
  DragRequest({
    required this.writer,
    required this.pointInRect,
    required this.image,
  });

  final RawClipboardWriter writer;
  final Offset pointInRect;
  final ui.Image image;

  Future<dynamic> serialize() async {
    final imageData = await ImageData.fromImage(image);
    return {
      'writerId': writer.handle,
      'pointInRect': pointInRect.serialize(),
      'image': imageData.serialize(),
    };
  }
}
