import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import '../reader.dart';
import '../reader_manager.dart';

class DataReaderHandleImpl {
  DataReaderHandleImpl._({
    required int handle,
    required FinalizableHandle finalizableHandle,
  })  : _handle = handle,
        _finalizableHandle = finalizableHandle;

  static DataReaderHandleImpl deserialize(dynamic handle) {
    final map = handle as Map;
    return DataReaderHandleImpl._(
      handle: map["handle"],
      finalizableHandle: map["finalizableHandle"],
    );
  }

  final int _handle;
  // ignore: unused_field
  final FinalizableHandle _finalizableHandle;
}

class DataReaderItemHandleImpl {
  DataReaderItemHandleImpl._({
    required int itemHandle,
    required DataReaderHandleImpl reader,
  })  : _itemHandle = itemHandle,
        _reader = reader;

  final int _itemHandle;
  int get _readerHandle => _reader._handle;

  // keep reader alive otherwise finalizable handle may dispose it
  final DataReaderHandleImpl _reader;
}

class ReaderManagerImpl extends ReaderManager {
  ReaderManagerImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  @override
  Future<void> dispose(DataReaderHandle reader) async {
    await _channel.invokeMethod("disposeReader", reader._handle);
  }

  @override
  Future<List<DataReaderItemHandleImpl>> getItems(
      DataReaderHandle reader) async {
    final handles =
        await _channel.invokeMethod("getItems", reader._handle) as List<int>;
    return handles
        .map((handle) => DataReaderItemHandle._(
              itemHandle: handle,
              reader: reader,
            ))
        .toList(growable: false);
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) async {
    final formats = await _channel.invokeMethod("getItemFormats", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
    }) as List;
    return formats.cast<String>();
  }

  @override
  Future<bool> itemFormatIsSynthetized(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    return _channel.invokeMethod("itemFormatIsSynthetized", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
    });
  }

  @override
  Future<String?> getItemSuggestedName(DataReaderItemHandle handle) async {
    final name = await _channel.invokeMethod("getItemSuggestedName", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
    }) as String?;
    return name;
  }

  @override
  Pair<Future<Object?>, ReadProgress> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final progress = ReadProgressImpl(readerManager: this);
    final completer = Completer<Object?>();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("getItemData", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
      "progressId": progress.id,
    }).then((value) {
      _completeProgress(progress.id);
      completer.complete(value);
    }, onError: (error) {
      _completeProgress(progress.id);
      completer.completeError(error);
    });
    return Pair(completer.future, progress);
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return await _channel.invokeMethod("canGetVirtualFile", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      'format': format,
    });
  }

  @override
  Pair<Future<String?>, ReadProgress> getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
  }) {
    final progress = ReadProgressImpl(readerManager: this);
    final completer = Completer<String?>();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("getVirtualFile", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
      'targetFolder': targetFolder,
      "progressId": progress.id,
    }).then((value) {
      _completeProgress(progress.id);
      completer.complete(value);
    }, onError: (error) {
      _completeProgress(progress.id);
      completer.completeError(error);
    });
    return Pair(completer.future, progress);
  }

  void _completeProgress(int progressId) {
    final progress = _progressMap.remove(progressId);
    if (progress != null) {
      progress._fraction.value = 1.0;
    }
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'setProgressCancellable') {
      final args = call.arguments as Map;
      final progressId = args['progressId'] as int;
      final cancellable = args['cancellable'] as bool;
      _progressMap[progressId]?._cancellable.value = cancellable;
    } else if (call.method == 'updateProgress') {
      final args = call.arguments as Map;
      final progressId = args['progressId'] as int;
      final fraction = args['fraction'] as double?;
      _progressMap[progressId]?._fraction.value = fraction;
    }
  }

  void cancelProgress(int progressId) {
    _channel.invokeMethod('cancelProgress', progressId);
  }

  final _channel = NativeMethodChannel('DataReaderManager',
      context: superNativeExtensionsContext);

  final _progressMap = <int, ReadProgressImpl>{};
}

class ReadProgressImpl extends ReadProgress {
  static int _nextId = 0;

  ReadProgressImpl({
    required this.readerManager,
  }) : id = _nextId++;

  final ReaderManagerImpl readerManager;

  final int id;

  @override
  void cancel() {
    readerManager.cancelProgress(id);
  }

  @override
  ValueListenable<double?> get fraction => _fraction;

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}
