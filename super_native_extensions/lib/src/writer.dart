import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/raw_clipboard.dart';

import 'context.dart';

class RawClipboardWriter {
  RawClipboardWriter._() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  static final instance = RawClipboardWriter._();

  Future<void> write(DataSourceHandle dataSource) async {
    await _channel.invokeMethod('writeToClipboard', dataSource.id);
    _activeSources[dataSource.id] = dataSource;
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'releaseDataSource') {
      final source = _activeSources.remove(call.arguments as int);
      if (source != null) {
        source.dispose();
      }
    }
  }

  final _channel = NativeMethodChannel('ClipboardWriter',
      context: superNativeExtensionsContext);

  final _activeSources = <int, DataSourceHandle>{};
}
