import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter/foundation.dart';
import 'package:web/web.dart' as web;

import '../clipboard_writer.dart';
import '../data_provider.dart';

class ClipboardWriterImpl extends ClipboardWriter {
  List<DataProviderHandle> _currentPayload = [];

  JSAny _toJS(Object? object) {
    if (object is String) {
      return object.toJS;
    } else if (object is Uint8List) {
      return object.toJS;
    } else {
      throw UnsupportedError('Unsupported data type: $object');
    }
  }

  web.ClipboardItem translateProvider(DataProvider provider) {
    final representations = JSObject();
    for (final repr in provider.representations) {
      if (repr.format == 'text/uri-list') {
        // Writing URI list to clipboard on web is not supported
        continue;
      }
      if (repr is DataRepresentationSimple) {
        final value = web.Blob(
          [_toJS(repr.data)].toJS,
          web.BlobPropertyBag(
            type: repr.format,
          ),
        );
        representations.setProperty(repr.format.toJS, value);
      } else if (repr is DataRepresentationLazy) {
        Future<web.Blob> fn() async {
          final data = await repr.dataProvider();
          return web.Blob(
            [_toJS(data)].toJS,
            web.BlobPropertyBag(
              type: repr.format,
            ),
          );
        }

        representations.setProperty(repr.format.toJS, fn().toJS);
      }
    }
    return web.ClipboardItem(representations);
  }

  @override
  Future<void> write(List<DataProviderHandle> providers) async {
    for (final handle in _currentPayload) {
      await handle.dispose();
    }
    _currentPayload = providers;
    final clipboard = web.window.navigator.clipboard;
    final items = providers.map((e) => translateProvider(e.provider));
    await clipboard.write(items.toList(growable: false).toJS).toDart;
  }
}
