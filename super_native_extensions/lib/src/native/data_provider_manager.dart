import 'dart:async';
import 'dart:ffi';
import 'dart:math';

import 'package:ffi/ffi.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import '../data_provider.dart';
import '../data_provider_manager.dart';
import '../util.dart';

// Needed for older Flutter versions :-/
// ignore: unnecessary_import
import 'dart:typed_data';

extension DataProviderExt on DataProvider {
  dynamic serialize() => {
        'representations': representations.map((e) => e.serialize()),
        'suggestedName': suggestedName,
      };
}

class DataProviderManagerImpl implements DataProviderManager {
  DataProviderManagerImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  @override
  Future<DataProviderHandle> registerDataProvider(DataProvider provider) async {
    final id = await _channel.invokeMethod(
        "registerDataProvider", provider.serialize());
    final handle = DataProviderHandle(id, provider);
    _handles[id] = handle;
    for (final representation in provider.representations) {
      if (representation is DataRepresentationLazy) {
        _lazyData[representation.id] = representation;
      } else if (representation is DataRepresentationVirtualFile) {
        _virtualFile[representation.id] = representation;
      }
    }

    return handle;
  }

  @override
  Future<void> unregisterDataProvider(int providerId) async {
    await _channel.invokeMethod("unregisterDataProvider", providerId);
    final handle = _handles.remove(providerId);
    if (handle != null) {
      for (final representation in handle.provider.representations) {
        if (representation is DataRepresentationLazy) {
          _lazyData.remove(representation.id);
        } else if (representation is DataRepresentationVirtualFile) {
          _virtualFile.remove(representation.id);
        }
      }
    }
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'getLazyData') {
      final args = call.arguments as Map;
      final valueId = args["valueId"] as int;
      final lazyData = _lazyData[valueId];
      if (lazyData != null) {
        return _ValuePromiseResult.ok(await lazyData.dataProvider())
            .serialize();
      } else {
        return _ValuePromiseResult.cancelled().serialize();
      }
    } else if (call.method == 'getVirtualFile') {
      final args = call.arguments;
      final sessionId = args['sessionId'] as int;
      final virtualFileId = args['virtualFileId'] as int;
      final fileHandle = args['streamHandle'] as int;
      return _getVirtualFile(
          sessionId: sessionId,
          virtualFileId: virtualFileId,
          streamHandle: fileHandle);
    } else if (call.method == 'cancelVirtualFile') {
      final sessionId = call.arguments as int;
      // Don't allow cancelling completed sessions. This can happen on
      // windows when IStream gets released by the client (which is what
      // triggers cancellation on windows) after having read entire length
      // before dart code hasn't yet closed the sink.
      if (_virtualSessions[sessionId]?.isCompleted != true) {
        final session = _virtualSessions.remove(sessionId);
        (session?.progress.onCancel as SimpleNotifier?)?.notify();
      }
    }
  }

  Future<dynamic> _getVirtualFile({
    required int sessionId,
    required int virtualFileId,
    required int streamHandle,
  }) async {
    final progressNotifier = ValueNotifier<double>(0.0);
    progressNotifier.addListener(() {
      _channel.invokeMethod('virtualFileUpdateProgress', {
        'sessionId': sessionId,
        'progress': progressNotifier.value,
      });
    });
    final progress = WriteProgressImpl(SimpleNotifier(), progressNotifier);
    final session = _VirtualSession(progress: progress);
    _virtualSessions[sessionId] = session;

    Future<void> onComplete() async {
      await _channel.invokeMethod('virtualFileComplete', {
        'sessionId': sessionId,
      });
      _virtualSessions.remove(sessionId);
    }

    Future<void> onError(String errorMessage) async {
      await _channel.invokeMethod('virtualFileError', {
        'sessionId': sessionId,
        'errorMessage': errorMessage,
      });
      _virtualSessions.remove(sessionId);
    }

    final sink = _VirtualFileSink(
      session: session,
      handle: streamHandle,
      onClose: onComplete,
      onError: onError,
    );

    final virtualFile = _virtualFile[virtualFileId];
    if (virtualFile != null) {
      EventSink provider({required int fileSize}) {
        if (session._fileSize != null && session._fileSize != fileSize) {
          throw StateError("File size can not be changed");
        }
        if (session._fileSize == null) {
          _channel.invokeMethod("virtualFileSizeKnown", {
            'sessionId': sessionId,
            'fileSize': fileSize,
          });
          session._fileSize = fileSize;
        }
        return sink;
      }

      virtualFile.virtualFileProvider(provider, progress);
    } else {
      onError('Virtual file ($virtualFileId)not found');
    }
    progress.onCancel.addListener(() async {
      sink._close(delete: true);
      await _channel.invokeMethod('virtualFileCancel', {
        'sessionId': sessionId,
      });
    });
    return null;
  }

  final _channel = NativeMethodChannel('DataProviderManager',
      context: superNativeExtensionsContext);

  final _handles = <int, DataProviderHandle>{};
  final _lazyData = <int, DataRepresentationLazy>{};
  final _virtualFile = <int, DataRepresentationVirtualFile>{};
  final _virtualSessions = <int, _VirtualSession>{};
}

class WriteProgressImpl extends WriteProgress {
  WriteProgressImpl(Listenable onCancel, ValueNotifier<double> onProgress)
      : _onCancel = onCancel,
        _onProgress = onProgress;

  @override
  void updateProgress(double fraction) {
    _onProgress.value = fraction;
    _hasExplicitProgress = true;
  }

  @override
  Listenable get onCancel => _onCancel;

  /// Used to update progress based on bytes written to stream.
  /// Only used unless user manually calls updateProgress.
  void updateProgressImplicit(double fraction) {
    if (_hasExplicitProgress) {
      return;
    }
    // throttle updates
    if (fraction == 1 || (fraction - _onProgress.value).abs() >= 0.05) {
      _onProgress.value = fraction;
    }
  }

  bool _hasExplicitProgress = false;
  final Listenable _onCancel;
  final ValueNotifier<double> _onProgress;
}

class _VirtualSession {
  _VirtualSession({
    required this.progress,
  });

  int? _fileSize;
  int _bytesWritten = 0;
  final WriteProgressImpl progress;

  void didWriteBytes(int bytes) {
    _bytesWritten += bytes;
    final fileSize = _fileSize ?? 0;
    if (fileSize > 0) {
      progress.updateProgressImplicit(_bytesWritten / fileSize);
    }
  }

  bool get isCompleted => _bytesWritten == _fileSize;
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
  final _VirtualSession session;
  final int handle;
  Pointer<Uint8>? _buffer;
  Future<void> Function() onClose;
  Future<void> Function(String) onError;

  _VirtualFileSink({
    required this.session,
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
      session.didWriteBytes(len);
      numWritten += len;
    }
  }

  void _close({
    bool delete = false,
  }) {
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
    _closed = true;
    await onError(error.toString());
    _close(delete: true);
  }

  @override
  Future<void> close() async {
    if (_closed) {
      return;
    }
    _closed = true;
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
