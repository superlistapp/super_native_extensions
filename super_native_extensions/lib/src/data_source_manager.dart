import 'dart:async';
import 'dart:ffi';
import 'dart:math';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'data_source.dart';
import 'util.dart';

class DataSourceManager {
  DataSourceManager._() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<DataSourceHandle> registerDataSource(DataSource source) async {
    final id =
        await _channel.invokeMethod("registerDataSource", source.serialize());
    final handle = DataSourceHandle(id, source);
    _handles[id] = handle;
    for (final item in handle.source.items) {
      for (final data in item.representations) {
        if (data is DataSourceItemRepresentationLazy) {
          _lazyData[data.id] = data;
        } else if (data is DataSourceItemRepresentationVirtualFile) {
          _virtualFile[data.id] = data;
        }
      }
    }
    return handle;
  }

  Future<void> unregisterDataSource(int sourceId) async {
    await _channel.invokeMethod("unregisterDataSource", sourceId);
    final handle = _handles.remove(sourceId);
    if (handle != null) {
      for (final item in handle.source.items) {
        for (final data in item.representations) {
          if (data is DataSourceItemRepresentationLazy) {
            _lazyData.remove(data.id);
          } else if (data is DataSourceItemRepresentationVirtualFile) {
            _virtualFile.remove(data.id);
          }
        }
      }
    }
  }

  Future<dynamic> getVirtualFile({
    required int sessionId,
    required int virtualFileId,
    required int streamHandle,
  }) async {
    final progressNotifier = ValueNotifier<int>(0);
    progressNotifier.addListener(() {
      _channel.invokeMethod('virtualFileUpdateProgress', {
        'sessionId': sessionId,
        'progress': progressNotifier.value,
      });
    });
    final progress = Progress(SimpleNotifier(), progressNotifier);
    _progressMap[sessionId] = progress;

    Future<void> onComplete() async {
      await _channel.invokeMethod('virtualFileComplete', {
        'sessionId': sessionId,
      });
      _progressMap.remove(sessionId);
    }

    Future<void> onError(String errorMessage) async {
      await _channel.invokeMethod('virtualFileError', {
        'sessionId': sessionId,
        'errorMessage': errorMessage,
      });
      _progressMap.remove(sessionId);
    }

    final sink = _VirtualFileSink(
        handle: streamHandle, onClose: onComplete, onError: onError);

    final virtualFile = _virtualFile[virtualFileId];
    if (virtualFile != null) {
      virtualFile.virtualFileProvider(sink, progress);
    } else {
      onError('Virtual file ($virtualFileId)not found');
    }
    progress.onCancel.addListener(() {
      sink._close(delete: true);
    });
    return null;
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'getLazyData') {
      final args = call.arguments as Map;
      final valueId = args["valueId"] as int;
      final format = args["format"] as String;
      final lazyData = _lazyData[valueId];
      if (lazyData != null) {
        return _ValuePromiseResult.ok(await lazyData.dataProvider(format))
            .serialize();
      } else {
        return _ValuePromiseResult.cancelled().serialize();
      }
    } else if (call.method == 'getVirtualFile') {
      final args = call.arguments;
      final sessionId = args['sessionId'] as int;
      final virtualFileId = args['virtualFileId'] as int;
      final fileHandle = args['fileHandle'] as int;
      return getVirtualFile(
          sessionId: sessionId,
          virtualFileId: virtualFileId,
          streamHandle: fileHandle);
    } else if (call.method == 'cancelVirtualFile') {
      final sessionId = call.arguments as int;
      final progress = _progressMap.remove(sessionId);
      if (progress != null) {
        (progress.onCancel as SimpleNotifier).notify();
      }
    }
  }

  static final instance = DataSourceManager._();

  final _channel = NativeMethodChannel('DataSourceManager',
      context: superNativeExtensionsContext);

  final _handles = <int, DataSourceHandle>{};
  final _lazyData = <int, DataSourceItemRepresentationLazy>{};
  final _virtualFile = <int, DataSourceItemRepresentationVirtualFile>{};
  final _progressMap = <int, Progress>{};
}

class _NativeFunctions {
  _NativeFunctions({
    required this.streamWrite,
    required this.streamClose,
  });

  static _NativeFunctions? _instance;

  static _NativeFunctions get instance {
    if (_instance == null) {
      final dylib = openNativeLibrary();
      final streamWrite = dylib
          .lookup<NativeFunction<Int32 Function(Int32, Pointer<Uint8>, Int64)>>(
              'super_native_extensions_stream_write')
          .asFunction<int Function(int, Pointer<Uint8>, int)>();
      final streamClose = dylib
          .lookup<NativeFunction<Void Function(Int32, Bool)>>(
              'super_native_extensions_stream_close')
          .asFunction<void Function(int, bool)>();
      _instance = _NativeFunctions(
        streamWrite: streamWrite,
        streamClose: streamClose,
      );
    }
    return _instance!;
  }

  final int Function(int handle, Pointer<Uint8> data, int len) streamWrite;
  final void Function(int handle, bool delete) streamClose;
}

class _VirtualFileSink extends EventSink<Uint8List> {
  bool _closed = false;
  final int handle;
  Pointer<Uint8>? _buffer;
  Future<void> Function() onClose;
  Future<void> Function(String) onError;

  _VirtualFileSink({
    required this.handle,
    required this.onClose,
    required this.onError,
  });

  @override
  void add(data) {
    if (_closed) {
      throw StateError('Stream is already closed');
    }
    const bufferSize = 16384;
    _buffer ??= malloc.allocate(bufferSize);

    int numWritten = 0;
    while (numWritten < data.length) {
      final len = min(bufferSize, data.length - numWritten);
      _buffer!
          .asTypedList(bufferSize)
          .setRange(0, len, data.sublist(numWritten, numWritten + len));
      _NativeFunctions.instance.streamWrite(handle, _buffer!, len);
      numWritten += len;
    }
  }

  void _close({
    bool delete = false,
  }) {
    _closed = true;
    if (_buffer != null) {
      malloc.free(_buffer!);
      _buffer = null;
    }
    _NativeFunctions.instance.streamClose(handle, delete);
  }

  @override
  Future<void> addError(Object error, [StackTrace? stackTrace]) async {
    if (_closed) {
      return;
    }
    _close(delete: true);
    return onError(error.toString());
  }

  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _close();
    return onClose();
  }
}

abstract class _ValuePromiseResult {
  static _ValuePromiseResultOk ok(dynamic value) =>
      _ValuePromiseResultOk._(value);

  static _ValuePromiseResultCancelled cancelled() =>
      _ValuePromiseResultCancelled._();

  dynamic serialize();
}

class _ValuePromiseResultCancelled extends _ValuePromiseResult {
  _ValuePromiseResultCancelled._();

  @override
  serialize() => {
        'type': 'cancelled',
      };
}

class _ValuePromiseResultOk extends _ValuePromiseResult {
  _ValuePromiseResultOk._(this.value);

  final dynamic value;

  @override
  serialize() => {
        'type': 'ok',
        'value': value,
      };
}