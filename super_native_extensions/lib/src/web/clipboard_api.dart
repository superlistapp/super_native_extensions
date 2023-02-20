@JS()
library clipboard_api;

import 'dart:html';
import 'dart:typed_data';

import 'package:js/js.dart';
import 'dart:js_util' as js_util;

@JS()
@staticInterop
class EventTarget {
  external factory EventTarget();
}

@JS()
@staticInterop
class Clipboard implements EventTarget {
  external factory Clipboard();
}

extension ClipboardExtension on Clipboard {
  Future<Iterable<ClipboardItem>> read() async {
    final Future<Iterable<dynamic>> items =
        js_util.promiseToFuture(js_util.callMethod(this, 'read', []));
    return (await items).cast<ClipboardItem>();
  }

  Future<void> write(Iterable<ClipboardItem> data) => js_util.promiseToFuture(
      js_util.callMethod(this, 'write', [data.toList(growable: false)]));
}

Clipboard getClipboard() {
  return js_util.getProperty(window.navigator, 'clipboard') as Clipboard;
}

@JS()
@staticInterop
class ClipboardItem {
  external factory ClipboardItem(dynamic items);
}

extension BlobExt on Blob {
  Future<String?> text() =>
      js_util.promiseToFuture(js_util.callMethod(this, 'text', []));
  Future<ByteBuffer?> arrayBuffer() =>
      js_util.promiseToFuture(js_util.callMethod(this, 'arrayBuffer', []));
}

extension ClipboardItemExtension on ClipboardItem {
  // One day... (right now no browser seem to support this)
  // static ClipboardItem createDelayed(
  //   dynamic items,
  // ) =>
  //     js_util.callMethod(ClipboardItem, 'createDelayed', [items]);

  Iterable<String> get types =>
      (js_util.getProperty(this, 'types') as Iterable).cast<String>();
  Future<Blob> getType(String type) =>
      js_util.promiseToFuture(js_util.callMethod(this, 'getType', [type]));
}
