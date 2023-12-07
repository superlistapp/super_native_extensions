import 'dart:html' as html;

import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'clipboard_api.dart';
import 'drop.dart';
import 'reader.dart';
import 'reader_manager.dart';

class _PasteEvent extends PasteEvent {
  _PasteEvent({
    required this.reader,
    required this.event,
  });

  final DataReader reader;
  final html.Event event;

  bool _defaultPrevented = false;

  @override
  DataReader getReader() {
    if (!_defaultPrevented) {
      _defaultPrevented = true;
      event.preventDefault();
    }
    return reader;
  }
}

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

  final _pasteEventListeners = <void Function(PasteEvent reader)>[];

  @override
  void registerPasteEventListener(void Function(PasteEvent reader) listener) {
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
        final pasteEvent = _PasteEvent(reader: reader, event: event);
        for (final listener in _pasteEventListeners) {
          listener(pasteEvent);
        }
      });
    }
  }

  @override
  void unregisterPasteEventListener(void Function(PasteEvent reader) listener) {
    _pasteEventListeners.remove(listener);
  }

  @override
  bool get supportsPasteEvent => true;
}
