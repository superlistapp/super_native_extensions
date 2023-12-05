import 'dart:html' as html;

import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'clipboard_api.dart';
import 'drop.dart';
import 'reader_manager.dart';

class ClipboardReaderHandle extends DataReaderItemHandleImpl {
  ClipboardReaderHandle(this.item);

  final ClipboardItem item;

  @override
  Future<List<String>> getFormats() async {
    return item.types.toList(growable: false);
  }

  @override
  Future<Object?> getDataForFormat(String format) async {
    final data = await item.getType(format);
    if (format.startsWith('text/')) {
      return data.text();
    } else {
      return (await data.arrayBuffer())?.asUint8List();
    }
  }

  @override
  Future<String?> suggestedName() async {
    // ClipboardItem can tell that it is an attachment but can not
    // provide name. Go figure.
    return null;
  }

  @override
  Future<bool> canGetVirtualFile(String format) async {
    return false;
  }

  @override
  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return null;
  }
}

class ClipboardReaderImpl extends ClipboardReader {
  @override
  Future<DataReader> newClipboardReader() async {
    final items = await getClipboard().read();
    final handle = DataReaderHandleImpl(
      items
          .map(
            (e) => ClipboardReaderHandle(e),
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
        final readerHandle = DataReaderHandleImpl(
          translated.map(
            (e) {
              return e.$2 as DataReaderItemHandleImpl;
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
