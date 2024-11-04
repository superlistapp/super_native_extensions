import 'dart:async';
import 'dart:js_interop';
import 'dart:typed_data';

import '../data_provider.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'js_interop.dart';
import 'reader_manager.dart';
import 'package:web/web.dart' as web;

/// Item handle that simply returns the data from the DataProvider.
/// This is used when dropping within same browser tab.
class DataProviderItemHandle extends $DataReaderItemHandle {
  DataProviderItemHandle(this.provider);

  final DataProviderHandle provider;

  @override
  Future<Object?> getDataForFormat(String format) async {
    for (final representation in provider.provider.representations) {
      if (representation is DataRepresentationSimple) {
        if (representation.format == format) {
          return representation.data;
        }
      } else if (representation is DataRepresentationLazy) {
        if (representation.format == format) {
          return representation.dataProvider();
        }
      }
    }
    return null;
  }

  @override
  Future<List<String>> getFormats() async {
    return provider.provider.representations
        .map((e) => e.format)
        .toList(growable: false);
  }

  @override
  Future<String?> suggestedName() async {
    return provider.provider.suggestedName;
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

/// Item handle backed by [web.ClipboardItem]. Used when interfacing with the
/// clipboard.
class ClipboardItemHandle extends $DataReaderItemHandle {
  ClipboardItemHandle(this.item);

  final web.ClipboardItem item;

  @override
  Future<List<String>> getFormats() async {
    return item.types.toDart.map((t) => t.toDart).toList(growable: false);
  }

  @override
  Future<Object?> getDataForFormat(String format) async {
    final data = await item.getType(format).toDart;
    if (format.startsWith('text/')) {
      return (await data.text().toDart).toDart;
    } else {
      return (await data.arrayBuffer().toDart).toDart.asUint8List();
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

/// ItemHandle backed by a list of [web.DataTransferItem]s.
class DataTransferItemHandle implements $DataReaderItemHandle {
  DataTransferItemHandle(this.items, {required bool canRead})
      : file = canRead ? _getFile(items) : null,
        entry = canRead ? _getEntry(items) : null,
        // reading strings multiple times fails in Chrome so we cache them
        strings = canRead ? _getStrings(items) : {};

  static web.File? _getFile(List<web.DataTransferItem> items) {
    for (final item in items) {
      if (item.isFile) {
        return item.getAsFile();
      }
    }
    return null;
  }

  static web.FileSystemEntry? _getEntry(List<web.DataTransferItem> items) {
    for (final item in items) {
      if (item.isFile) {
        return item.webkitGetAsEntry();
      }
    }
    return null;
  }

  static Map<String, Future<String>> _getStrings(
      List<web.DataTransferItem> items) {
    final res = <String, Future<String>>{};
    for (final item in items) {
      if (item.isString) {
        final completer = Completer<String>();
        void complete(JSString string) {
          completer.complete(string.toDart);
        }

        item.getAsString(complete.toJS);
        res[item.format] = completer.future;
      }
    }
    return res;
  }

  @override
  Future<Object?> getDataForFormat(String format) async {
    // meta-formats
    if (format == 'web:file') {
      return file;
    }
    if (format == 'web:entry') {
      return entry;
    }
    if (strings.containsKey(format)) {
      return strings[format];
    }
    for (final item in items) {
      if (item.isFile && item.format == format) {
        final file = this.file;
        if (file != null) {
          final slice = file.slice();
          final buffer = await slice.arrayBuffer().toDart;
          return buffer.toDart.asUint8List();
        }
      }
    }
    return Future.value(null);
  }

  List<String> getFormatsSync() {
    final formats = items.map((e) => e.format).toList(growable: true);
    // meta formats for file (web.File) and entry (web.Entry)
    if (file != null) {
      formats.add('web:file');
    }
    if (entry != null) {
      formats.add('web:entry');
    }
    // safari doesn't provide types during dragging, but we still need to report
    // to use that there is potential contents.
    return formats.isNotEmpty ? formats : ['web:unknown'];
  }

  @override
  Future<List<String>> getFormats() async {
    return getFormatsSync();
  }

  @override
  Future<String?> suggestedName() async {
    return file?.name;
  }

  final web.File? file;
  final web.FileSystemEntry? entry;
  final Map<String, Future<String>> strings;
  final List<web.DataTransferItem> items;

  @override
  Future<bool> canGetVirtualFile(String format) async {
    return !format.startsWith('web:') && file != null;
  }

  @override
  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    if (await canGetVirtualFile(format)) {
      return _VirtualFileReceiver(format, file!);
    } else {
      return null;
    }
  }
}

class _VirtualFileReceiver extends VirtualFileReceiver {
  _VirtualFileReceiver(this.format, this.file);

  @override
  final String format;
  final web.File file;

  @override
  (Future<String>, ReadProgress) copyVirtualFile(
      {required String targetFolder}) {
    throw UnimplementedError();
  }

  @override
  (Future<VirtualFile>, ReadProgress) receiveVirtualFile() {
    final progress = SimpleProgress();
    progress.done();
    return (Future.value(_VirtualFile(file)), progress);
  }
}

class _VirtualFile extends VirtualFile {
  _VirtualFile(this.file);

  final web.File file;

  web.ReadableStreamDefaultReader? _reader;

  @override
  void close() {
    _reader?.cancel();
  }

  @override
  String? get fileName => file.name;

  @override
  int? get length => file.size;

  @override
  Future<Uint8List> readNext() async {
    if (_reader == null) {
      final stream = file.stream();
      _reader = web.ReadableStreamDefaultReader(stream);
    }

    final next = await _reader!.read().toDart;
    if (next.done) {
      return Uint8List(0);
    } else {
      return (next.value as JSUint8Array).toDart;
    }
  }
}
