import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'reader.dart';

class DataReaderHandle {
  DataReaderHandle._({
    required int handle,
    required FinalizableHandle finalizableHandle,
  })  : _handle = handle,
        _finalizableHandle = finalizableHandle;

  static DataReaderHandle deserialize(dynamic handle) {
    final map = handle as Map;
    return DataReaderHandle._(
      handle: map["handle"],
      finalizableHandle: map["finalizableHandle"],
    );
  }

  final int _handle;
  // ignore: unused_field
  final FinalizableHandle _finalizableHandle;
}

class DataReaderItemHandle {
  DataReaderItemHandle._({
    required int itemHandle,
    required int readerHandle,
  })  : _itemHandle = itemHandle,
        _readerHandle = readerHandle;

  final int _itemHandle;
  final int _readerHandle;
}

class RawReaderManager {
  RawReaderManager._() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<void> dispose(DataReaderHandle reader) async {
    await _channel.invokeMethod("disposeReader", reader._handle);
  }

  Future<List<DataReaderItem>> getItems(DataReaderHandle reader) async {
    final handles =
        await _channel.invokeMethod("getItems", reader._handle) as List<int>;
    return handles
        .map((handle) => DataReaderItem(
            handle: DataReaderItemHandle._(
                itemHandle: handle, readerHandle: reader._handle)))
        .toList(growable: false);
  }

  Future<List<String>> getItemFormats(DataReaderItemHandle handle) async {
    final formats = await _channel.invokeMethod("getItemFormats", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
    }) as List;
    return formats.cast<String>();
  }

  ReadProgress getItemData(
    DataReaderItemHandle handle, {
    required String format,
    required ValueChanged<GetDataResult> onData,
  }) {
    final progress = ReadProgressImpl();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("getItemData", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
      "progressId": progress.id,
    }).then((value) {
      _completeProgress(progress.id);
      onData(GetDataResult(value, null));
    }, onError: (error) {
      _completeProgress(progress.id);
      onData(GetDataResult(null, error));
    });
    return progress;
  }

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

  ReadProgress getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
    required ValueChanged<DataResult<String?>> onResult,
  }) {
    final progress = ReadProgressImpl();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("getVirtualFile", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
      'targetFolder': targetFolder,
      "progressId": progress.id,
    }).then((value) {
      _completeProgress(progress.id);
      onResult(DataResult(value, null));
    }, onError: (error) {
      _completeProgress(progress.id);
      onResult(DataResult(null, error));
    });
    return progress;
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

  static final instance = RawReaderManager._();
}

class ReadProgressImpl extends ReadProgress {
  static int _nextId = 0;

  ReadProgressImpl() : id = _nextId++;

  final int id;

  @override
  void cancel() {
    RawReaderManager.instance.cancelProgress(id);
  }

  @override
  ValueListenable<double?> get fraction => _fraction;

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}
