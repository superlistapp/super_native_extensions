import 'dart:html' as html;

import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'clipboard_api.dart';
import 'drop.dart';
import 'reader.dart';
import 'reader_manager.dart';

class ClipboardReaderImpl extends ClipboardReader {
  @override
  Future<DataReader> newClipboardReader() async {
    final items = await getClipboard().read();
    final handle = $DataReaderHandle(
      items
          .map(
            (e) => ClipboardItemHandle(e),
          )
          .toList(growable: false),
    );

    return DataReader(handle: handle as DataReaderHandle);
  }

  bool _pasteEventRegistered = false;

  final _pasteEventListeners = <void Function(DataReader reader)>[];

  @override
  void registerPasteEventListener(void Function(DataReader reader) listener) {
    _pasteEventListeners.add(listener);
    if (!_pasteEventRegistered) {
      _pasteEventRegistered = true;
      html.window.addEventListener('paste', (event) {
        final clipboardEvent = event as html.ClipboardEvent;
        final itemList = clipboardEvent.clipboardData?.items;
        if (itemList == null) {
          return;
        }
        final translated = translateDataTransfer(clipboardEvent.clipboardData!,
            allowReader: true);
        final readerHandle = $DataReaderHandle(
          translated.map(
            (e) {
              return e.$2 as $DataReaderItemHandle;
            },
          ).toList(growable: false),
        );
        final reader = DataReader(handle: readerHandle as DataReaderHandle);
        for (final listener in _pasteEventListeners) {
          listener(reader);
        }
      });
    }
  }

  @override
  void unregisterPasteEventListener(void Function(DataReader reader) listener) {
    _pasteEventListeners.remove(listener);
  }

  @override
  bool get supportsPasteEvent => true;
}
