import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import '../data_provider.dart';
import '../clipboard_writer.dart';

class ClipboardWriterImpl extends ClipboardWriter {
  ClipboardWriterImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  @override
  Future<void> write(List<DataProviderHandle> providers) async {
    await _channel.invokeMethod('writeToClipboard', providers.map((e) => e.id));
    for (final provider in providers) {
      _activeProviders[provider.id] = provider;
    }
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'releaseDataProvider') {
      final provider = _activeProviders.remove(call.arguments as int);
      if (provider != null) {
        provider.dispose();
      }
    }
  }

  final _channel = NativeMethodChannel('ClipboardWriter',
      context: superNativeExtensionsContext);

  final _activeProviders = <int, DataProviderHandle>{};
}
