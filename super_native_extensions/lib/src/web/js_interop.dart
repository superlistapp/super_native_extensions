import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:web/web.dart' as web;

extension DataTransferItemListExt on web.DataTransferItemList {
  external web.DataTransferItem operator [](int index);
}

extension DataTransferItemExt on web.DataTransferItem {
  bool get isString => kind == 'string';
  bool get isFile => kind == 'file';

  String get format {
    final type = this.type;
    if (type.isNotEmpty) {
      return type;
    } else if (isString) {
      return 'text/plain';
    } else {
      return 'application/octet-stream';
    }
  }
}

bool get clipboardItemAvailable {
  return web.window.getProperty('ClipboardItem'.toJS) != null;
}
