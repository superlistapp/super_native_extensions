import 'dart:async';
import 'dart:html' as html;
import 'dart:typed_data';
import 'dart:js_util' as js_util;

import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';

import '../mutex.dart';
import '../api_model.dart';
import '../data_provider.dart';
import '../drag.dart';
import '../drop.dart';
import '../reader.dart';
import '../reader_manager.dart';

import 'js_interop.dart';
import 'clipboard_api.dart';
import 'reader_manager.dart';

class WebItemDataReaderHandle implements DataReaderItemHandleImpl {
  WebItemDataReaderHandle(this.items, {required bool canRead})
      : file = canRead ? _getFile(items) : null,
        entry = canRead ? _getEntry(items) : null,
        // reading strings multiple times fails in Chrome so we cache them
        strings = canRead ? _getStrings(items) : {};

  static html.File? _getFile(List<html.DataTransferItem> items) {
    for (final item in items) {
      if (item.isFile) {
        return item.getAsFile();
      }
    }
    return null;
  }

  static html.Entry? _getEntry(List<html.DataTransferItem> items) {
    for (final item in items) {
      if (item.isFile) {
        return item.getAsEntry();
      }
    }
    return null;
  }

  static Map<String, Future<String>> _getStrings(
      List<html.DataTransferItem> items) {
    final res = <String, Future<String>>{};
    for (final item in items) {
      if (item.isString) {
        final completer = Completer<String>();
        item.getAsString(completer.complete);
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
          final buffer = await slice.arrayBuffer();
          return buffer?.asUint8List();
        }
      }
    }
    return Future.value(null);
  }

  List<String> getFormatsSync() {
    final formats = items.map((e) => e.format).toList(growable: true);
    // meta formats for file (html.File) and entry (html.Entry)
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

  final html.File? file;
  final html.Entry? entry;
  final Map<String, Future<String>> strings;
  final List<html.DataTransferItem> items;

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
  final html.File file;

  @override
  Pair<Future<String>, ReadProgress> copyVirtualFile(
      {required String targetFolder}) {
    throw UnimplementedError();
  }

  @override
  Pair<Future<VirtualFile>, ReadProgress> receiveVirtualFile() {
    final progress = SimpleProgress();
    progress.done();
    return Pair(Future.value(_VirtualFile(file)), progress);
  }
}

class _VirtualFile extends VirtualFile {
  _VirtualFile(this.file);

  final html.File file;

  ReadableStreamDefaultReader? _reader;

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
      _reader = stream.getReader();
    }

    final next = await _reader!.read();
    if (next.done) {
      return Uint8List(0);
    } else {
      return next.value as Uint8List;
    }
  }
}

List<DropItem> _translateTransferItems(
  html.DataTransferItemList? itemList, {
  required bool allowReader,
  required bool hasFiles,
}) {
  final res = <WebItemDataReaderHandle>[];
  var items = <html.DataTransferItem>[];

  for (int i = 0; i < (itemList?.length ?? 0); ++i) {
    final item = itemList![i];
    if ((item.isString && items.any((element) => element.type == item.type)) ||
        (item.isFile && items.any((element) => element.isFile))) {
      res.add(WebItemDataReaderHandle(items, canRead: allowReader));
      items = <html.DataTransferItem>[];
    }
    items.add(item);
  }
  if (items.isNotEmpty) {
    res.add(WebItemDataReaderHandle(items, canRead: allowReader));
  }
  if (res.isEmpty && hasFiles) {
    res.add(WebItemDataReaderHandle([], canRead: false));
  }
  return res
      .mapIndexed((i, e) => DropItem(
            itemId: i,
            formats: e.getFormatsSync(),
            readerItem: allowReader
                ? DataReaderItem(handle: e as DataReaderItemHandle)
                : null,
          ))
      .toList(growable: false);
}

List<DropOperation> _translateAllowedEffect(String? effects) {
  switch (effects?.toLowerCase()) {
    case 'copy':
      return [DropOperation.copy];
    case 'copylink':
      return [DropOperation.copy, DropOperation.link];
    case 'copymove':
      return [DropOperation.copy, DropOperation.move];
    case 'link':
      return [DropOperation.link];
    case 'linkmove':
      return [DropOperation.link, DropOperation.move];
    case 'move':
      return [DropOperation.move];
    case 'all':
    case 'uninitialized':
      return [DropOperation.copy, DropOperation.link, DropOperation.move];
    default:
      return [];
  }
}

extension ToWeb on DropOperation {
  String toWeb() {
    switch (this) {
      case DropOperation.copy:
        return 'copy';
      case DropOperation.move:
        return 'move';
      case DropOperation.link:
        return 'link';
      default:
        return 'none';
    }
  }
}

class DropContextImpl extends DropContext {
  static DropContextImpl? instance;

  DropContextImpl() {
    instance = this;
  }

  final _mutex = Mutex();

  int _nextSessionId = 1;
  int? _sessionId;
  var lastOperation = DropOperation.none;

  void _onDragEnter(html.DataTransfer transfer, html.MouseEvent event) {
    _sessionId = _nextSessionId++;
    _onDragOver(transfer, event);
  }

  void _onDragOver(html.DataTransfer transfer, html.MouseEvent event) async {
    if (_sessionId == null) {
      return;
    }
    final bool hasFiles = transfer.types?.contains("Files") ?? false;
    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateTransferItems(
        transfer.items,
        allowReader: false,
        hasFiles: hasFiles,
      ),
    );
    final currentSessionId = _sessionId;
    transfer.dropEffect = lastOperation.toWeb();

    final value = await _mutex.protect(() async {
      return await delegate?.onDropUpdate(dropEvent);
    });

    if (_sessionId == currentSessionId && value != null) {
      lastOperation = value;
    }
  }

  void _onDragLeave() async {
    lastOperation = DropOperation.none;
    final sessionId = _sessionId;
    _sessionId = null;
    if (sessionId != null) {
      await _mutex.protect(() async {
        await delegate?.onDropLeave(BaseDropEvent(sessionId: sessionId));
        await delegate?.onDropEnded(BaseDropEvent(sessionId: sessionId));
      });
    }
  }

  void _onDrop(html.DataTransfer transfer, html.MouseEvent event) async {
    final bool hasFiles = transfer.types?.contains("Files") ?? false;
    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateTransferItems(
        transfer.items,
        allowReader: true,
        hasFiles: hasFiles,
      ),
      acceptedOperation: lastOperation,
    );
    await _mutex.protect(() async {
      await delegate?.onPerformDrop(dropEvent);
    });
    _onDragLeave();
  }

  @override
  Future<void> initialize() async {
    html.document.addEventListener('dragenter', (event) {
      event.preventDefault();
      final dataTransfer =
          js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
      _onDragEnter(dataTransfer, event as html.MouseEvent);
    });
    html.document.addEventListener('dragover', (event) {
      event.preventDefault();
      final dataTransfer =
          js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
      _onDragOver(dataTransfer, event as html.MouseEvent);
    });
    html.document.addEventListener('drop', (event) async {
      event.preventDefault();
      final dataTransfer =
          js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
      _onDrop(dataTransfer, event as html.MouseEvent);
    });
    html.document.addEventListener('dragleave', (event) {
      event.preventDefault();
      _onDragLeave();
    });
  }

  @override
  Future<void> registerDropFormats(List<String> formats) async {}

  DropEvent _createLocalDropEvent({
    required DragConfiguration configuration,
    required Offset position,
    DropOperation? acceptedOperation,
  }) {
    List<String> itemFormats(DataProviderHandle handle) {
      // filter duplicates
      final have = <String>{};
      return handle.provider.representations
          .map((e) => e.format)
          .toList(growable: true)
        ..retainWhere((e) => have.add(e));
    }

    return DropEvent(
      sessionId: identityHashCode(configuration),
      locationInView: position,
      allowedOperations: configuration.allowedOperations,
      items: configuration.items
          .map(
            (item) => DropItem(
              itemId: identityHashCode(item),
              formats: itemFormats(item.dataProvider),
              localData: item.localData,
              readerItem: DataReaderItem(
                handle: DataProviderReaderItem(item.dataProvider)
                    as DataReaderItemHandle,
              ),
            ),
          )
          .toList(growable: false),
      acceptedOperation: acceptedOperation,
    );
  }

  Future<DropOperation> localSessionDidMove(
    DragConfiguration configuration,
    Offset position,
  ) async {
    return _mutex.protect(() async {
      return await delegate?.onDropUpdate(
            _createLocalDropEvent(
              configuration: configuration,
              position: position,
            ),
          ) ??
          DropOperation.none;
    });
  }

  Future<void> localSessionDrop(
    DragConfiguration configuration,
    Offset position,
    DropOperation acceptedOperation,
  ) async {
    return _mutex.protect(() async {
      await delegate?.onPerformDrop(
        _createLocalDropEvent(
          configuration: configuration,
          position: position,
          acceptedOperation: acceptedOperation,
        ),
      );
    });
  }

  void localSessionDidEnd(DragConfiguration configuration) {
    _mutex.protect(() async {
      final event = BaseDropEvent(sessionId: identityHashCode(configuration));
      await delegate?.onDropLeave(event);
      await delegate?.onDropEnded(event);
    });
  }
}
