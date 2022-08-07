import 'dart:async';
import 'dart:html' as html;

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:js/js.dart';

import '../api_model.dart';
import '../data_provider.dart';
import '../drag.dart';
import '../drop.dart';
import '../reader.dart';
import '../reader_manager.dart';

import 'clipboard_api.dart';
import 'reader_manager.dart';

import 'dart:js_util' as js_util;

extension DataTransferItemExt on html.DataTransferItem {
  bool get isString => kind == 'string';
  bool get isFile => kind == 'file';

  String get format {
    if (type != null) {
      return type!;
    } else if (isString) {
      return 'text/plain';
    } else {
      return 'application/octet-stream';
    }
  }

  void getAsString(ValueChanged<String> callback) {
    js_util.callMethod(this, 'getAsString', [allowInterop(callback)]);
  }
}

class WebItemDataReaderHandle implements DataReaderItemHandleImpl {
  WebItemDataReaderHandle(this.items, {required bool canRead})
      : files = canRead ? _getFiles(items) : {},
        strings = canRead ? _getStrings(items) : {};

  static Map<String, html.File> _getFiles(List<html.DataTransferItem> items) {
    final res = <String, html.File>{};
    for (final item in items) {
      if (item.isFile) {
        res[item.format] = item.getAsFile()!;
      }
    }
    return res;
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
    for (final item in items) {
      if (item.format == format) {
        if (item.isFile) {
          final file = files[item.format];
          if (file != null) {
            final slice = file.slice();
            final buffer = await slice.arrayBuffer();
            return buffer?.asUint8List();
          }
        } else if (item.isString) {
          final string = strings[item.format];
          if (string != null) {
            return string;
          }
        }
      }
    }
    return Future.value(null);
  }

  List<String> getFormatsSync() {
    return items.map((e) => e.format).toList(growable: false);
  }

  @override
  Future<List<String>> getFormats() {
    return SynchronousFuture(getFormatsSync());
  }

  @override
  Future<String?> suggestedName() async {
    return files.entries.firstOrNull?.value.name;
  }

  final Map<String, html.File> files;
  final Map<String, Future<String>> strings;
  final List<html.DataTransferItem> items;
}

List<DropItem> _translateTransferItems(
  html.DataTransferItemList? itemList, {
  required bool allowReader,
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

  int _nextSessionId = 1;
  int? _sessionId;
  var lastOperation = DropOperation.none;

  void _onDragEnter(html.DataTransfer transfer, html.MouseEvent event) {
    _sessionId = _nextSessionId++;
    _onDragOver(transfer, event);
  }

  void _onDragOver(html.DataTransfer transfer, html.MouseEvent event) {
    if (_sessionId == null) {
      return;
    }
    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateTransferItems(transfer.items, allowReader: false),
    );
    final currentSessionId = _sessionId;
    delegate?.onDropUpdate(dropEvent).then((value) {
      if (_sessionId == currentSessionId) {
        lastOperation = value;
      }
    });
    transfer.dropEffect = lastOperation.toWeb();
  }

  void _onDragLeave() {
    if (_sessionId != null) {
      delegate?.onDropLeave(BaseDropEvent(sessionId: _sessionId!));
      delegate?.onDropEnded(BaseDropEvent(sessionId: _sessionId!));
      _sessionId = null;
    }
    lastOperation = DropOperation.none;
  }

  void _onDrop(html.DataTransfer transfer, html.MouseEvent event) async {
    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateTransferItems(transfer.items, allowReader: true),
      acceptedOperation: lastOperation,
    );
    await delegate?.onPerformDrop(dropEvent);
    _onDragLeave();
  }

  @override
  Future<void> initialize() async {
    html.document.addEventListener('dragover', (event) {
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
      return handle.provider.representations.map((e) => e.format).toList();
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
    return await delegate?.onDropUpdate(
          _createLocalDropEvent(
            configuration: configuration,
            position: position,
          ),
        ) ??
        DropOperation.none;
  }

  Future<void> localSessionDrop(
    DragConfiguration configuration,
    Offset position,
    DropOperation acceptedOperation,
  ) async {
    await delegate?.onPerformDrop(
      _createLocalDropEvent(
        configuration: configuration,
        position: position,
        acceptedOperation: acceptedOperation,
      ),
    );
  }

  void localSessionDidEnd(DragConfiguration configuration) {
    final event = BaseDropEvent(sessionId: identityHashCode(configuration));
    delegate?.onDropLeave(event);
    delegate?.onDropEnded(event);
  }
}
