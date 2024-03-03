import 'dart:js_interop';

import 'package:web/web.dart';

import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';

import 'js_interop.dart';
import 'reader.dart';
import 'reader_manager.dart';

class ClipboardReaderImpl extends ClipboardReader {
  @override
  Future<DataReader> newClipboardReader() async {
    final items = await window.navigator.clipboard.read().toDart;
    final handle = $DataReaderHandle(
      items.toDart
          .map(
            (e) => ClipboardItemHandle(e),
          )
          .toList(growable: false),
    );

    return DataReader(handle: handle as DataReaderHandle);
  }

  @override
  bool get available => clipboardItemAvailable;
}
