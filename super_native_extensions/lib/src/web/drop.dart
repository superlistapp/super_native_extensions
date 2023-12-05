import 'dart:async';
import 'dart:html' as html;

import 'dart:js_util' as js_util;

import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';

import '../mutex.dart';
import '../data_provider.dart';
import '../drag.dart';
import '../drop.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'js_interop.dart';
import 'reader.dart';

List<DropItem> _translateDataTransfer(
  html.DataTransfer dataTransfer, {
  required bool allowReader,
}) {
  return translateDataTransfer(dataTransfer, allowReader: allowReader)
      .mapIndexed((i, e) => DropItem(
            itemId: i,
            formats: e.$1,
            readerItem: allowReader
                ? DataReaderItem(handle: e.$2 as DataReaderItemHandle)
                : null,
          ))
      .toList(growable: false);
}

Iterable<(List<String> formats, $DataReaderItemHandle? readerHandle)>
    translateDataTransfer(
  html.DataTransfer dataTransfer, {
  required bool allowReader,
}) {
  final itemList = dataTransfer.items;
  final hasFiles = dataTransfer.types?.contains("Files") ?? false;

  final res = <DataTransferItemHandle>[];
  var items = <html.DataTransferItem>[];

  for (int i = 0; i < (itemList?.length ?? 0); ++i) {
    final item = itemList![i];
    if ((item.isString && items.any((element) => element.type == item.type)) ||
        (item.isFile && items.any((element) => element.isFile))) {
      res.add(DataTransferItemHandle(items, canRead: allowReader));
      items = <html.DataTransferItem>[];
    }
    items.add(item);
  }
  if (items.isNotEmpty) {
    res.add(DataTransferItemHandle(items, canRead: allowReader));
  }
  if (res.isEmpty && hasFiles) {
    res.add(DataTransferItemHandle([], canRead: false));
  }
  return res.map((e) => (e.getFormatsSync(), allowReader ? e : null));
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

    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateDataTransfer(
        transfer,
        allowReader: false,
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
    final dropEvent = DropEvent(
      sessionId: _sessionId!,
      locationInView: Offset(event.page.x.toDouble(), event.page.y.toDouble()),
      allowedOperations: _translateAllowedEffect(transfer.effectAllowed),
      items: _translateDataTransfer(
        transfer,
        allowReader: true,
      ),
      acceptedOperation: lastOperation,
    );
    await _mutex.protect(() async {
      await delegate?.onPerformDrop(dropEvent);
    });
    _onDragLeave();
  }

  /// Last element received dragEnter event. We ignore all dragLeave events
  /// from other elements because when using platform view the drag events
  /// are not propagated to parent elements.
  /// https://github.com/superlistapp/super_native_extensions/issues/98
  html.EventTarget? _lastDragEnter;

  @override
  Future<void> initialize() async {
    html.document.addEventListener('dragenter', (event) {
      final inProgress = _lastDragEnter != null;
      _lastDragEnter = event.target;
      event.preventDefault();
      if (!inProgress) {
        final dataTransfer =
            js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
        _onDragEnter(dataTransfer, event as html.MouseEvent);
      }
    });
    html.document.addEventListener('dragover', (event) {
      event.preventDefault();
      final dataTransfer =
          js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
      _onDragOver(dataTransfer, event as html.MouseEvent);
    });
    html.document.addEventListener('drop', (event) async {
      event.preventDefault();
      _lastDragEnter = null;
      final dataTransfer =
          js_util.getProperty(event, 'dataTransfer') as html.DataTransfer;
      _onDrop(dataTransfer, event as html.MouseEvent);
    });
    html.document.addEventListener('dragleave', (event) {
      if (_lastDragEnter != event.target) {
        return;
      } else {
        _lastDragEnter = null;
      }
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
                handle: DataProviderItemHandle(item.dataProvider)
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
