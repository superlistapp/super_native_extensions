import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import 'context.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'virtual_file.dart';

class $DataReaderHandle {
  $DataReaderHandle._({
    required int handle,
    required FinalizableHandle finalizableHandle,
  })  : _handle = handle,
        _finalizableHandle = finalizableHandle {
    // In release mode the garbage collector eagerly disposes
    // _finalizableHandle even if the surrounding DataReaderHandleImpl
    // is still reachable.
    // This is a workaround to keep the handle alive.
    _useHandle();
  }

  static $DataReaderHandle deserialize(dynamic handle) {
    final map = handle as Map;
    return $DataReaderHandle._(
      handle: map["handle"],
      finalizableHandle: map["finalizableHandle"],
    );
  }

  bool _disposed = false;

  void _useHandle() {
    // This will always be false but it's enough to make the garbage collector
    // think we're using the handle.
    if (_finalizableHandle as dynamic == null) {
      _useHandle();
    }
  }

  final int _handle;
  final FinalizableHandle _finalizableHandle;
}

class $DataReaderItemHandle {
  $DataReaderItemHandle._({
    required int itemHandle,
    required $DataReaderHandle reader,
  })  : _itemHandle = itemHandle,
        _reader = reader;

  final int _itemHandle;
  int get _readerHandle => _reader._handle;

  // keep reader alive otherwise finalizable handle may dispose it
  final $DataReaderHandle _reader;
}

class ReaderManagerImpl extends ReaderManager {
  ReaderManagerImpl() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  @override
  Future<void> dispose(DataReaderHandle reader) async {
    if (!reader._disposed) {
      reader._disposed = true;
      await _channel.invokeMethod("disposeReader", reader._handle);
    }
  }

  @override
  Future<List<$DataReaderItemHandle>> getItems(DataReaderHandle reader) async {
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
  Future<bool> itemFormatIsSynthesized(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    if (handle._reader._disposed) {
      throw StateError("Attempting to query item status from disposed reader.");
    }
    return _channel.invokeMethod("itemFormatIsSynthesized", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
    });
  }

  @override
  Future<String?> getItemSuggestedName(DataReaderItemHandle handle) async {
    if (handle._reader._disposed) {
      throw StateError(
          "Attempting to get suggested name from disposed reader.");
    }
    final name = await _channel.invokeMethod("getItemSuggestedName", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
    }) as String?;
    return name;
  }

  @override
  (Future<Object?>, ReadProgress) getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    if (handle._reader._disposed) {
      throw StateError("Attempting to get data from disposed reader.");
    }
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
    return (completer.future, progress);
  }

  Future<bool> canCopyVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    if (handle._reader._disposed) {
      throw StateError(
          "Attempting to query virtual file from disposed reader.");
    }
    return await _channel.invokeMethod("canCopyVirtualFile", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      'format': format,
    });
  }

  Future<bool> canReadVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    if (handle._reader._disposed) {
      throw StateError(
          "Attempting to query virtual file from disposed reader.");
    }
    return await _channel.invokeMethod("canReadVirtualFile", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      'format': format,
    });
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return (await canReadVirtualFile(handle, format: format)) ||
        (await canCopyVirtualFile(handle, format: format));
  }

  @override
  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    // First try to produce receiver that can receive the file without copying
    // it first
    if (await canReadVirtualFile(handle, format: format)) {
      assert(
          await canCopyVirtualFile(handle, format: format),
          'If implementation can read virtual file it must also '
          'be able to copy virtual file.');
      return _VirtualFileReceiver(
        readerManager: this,
        handle: handle,
        format: format,
      );
    } else if (await canCopyVirtualFile(handle, format: format)) {
      return _CopyVirtualFileReceiver(
        readerManager: this,
        handle: handle,
        format: format,
      );
    } else {
      return null;
    }
  }

  (Future<VirtualFile>, ReadProgress) virtualFileCreate(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    if (handle._reader._disposed) {
      throw StateError("Attempting to get virtual file from disposed reader.");
    }
    final progress = ReadProgressImpl(readerManager: this);
    final completer = Completer<VirtualFile>();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("virtualFileReaderCreate", {
      "itemHandle": handle._itemHandle,
      "readerHandle": handle._readerHandle,
      "format": format,
      "progressId": progress.id,
    }).then((value) {
      _completeProgress(progress.id);
      final response = value as Map;
      final file = _VirtualFile(
        readerManager: this,
        handle: response['readerHandle'],
        fileName: response['fileName'],
        length: response['fileSize'],
      );
      completer.complete(file);
    }, onError: (error) {
      _completeProgress(progress.id);
      completer.completeError(error);
    });
    return (completer.future, progress);
  }

  @override
  Future<String?> formatForFileUri(Uri uri) {
    return _channel.invokeMethod('getFormatForFileUri', uri.toString());
  }

  Future<Uint8List?> virtualFileRead({
    required int handle,
  }) {
    return _channel.invokeMethod('virtualFileReaderRead', handle);
  }

  Future<void> virtualFileClose({
    required int handle,
  }) async {
    await _channel.invokeMethod('virtualFileReaderClose', handle);
  }

  (Future<String>, ReadProgress) copyVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
  }) {
    if (handle._reader._disposed) {
      throw StateError("Attempting to get virtual file from disposed reader.");
    }
    final progress = ReadProgressImpl(readerManager: this);
    final completer = Completer<String>();
    _progressMap[progress.id] = progress;
    _channel.invokeMethod("copyVirtualFile", {
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
    return (completer.future, progress);
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

  @override
  VirtualFile createVirtualFileFromUri(Uri uri) {
    final file = File(uri.toFilePath());
    return VirtualFileFromFile(file: file, onClose: () {});
  }

  @override
  Future<List<DataReaderItemInfo>> getItemInfo(
      Iterable<DataReaderItemHandle> handles) async {
    if (handles.isEmpty) {
      return [];
    }

    final reader = handles.first._reader;

    final handleMap =
        Map.fromEntries(handles.map((e) => MapEntry(e._itemHandle, e)));
    final res_ = await _channel.invokeMethod('getItemInfo', {
      'readerHandle': reader._handle,
      'itemHandles': handles.map((e) => e._itemHandle),
    });
    final list = res_['items'] as List;
    final res = list.map((e) {
      final handle = handleMap[e['handle']]!;
      final virtualFormats = (e['virtualFileFormats'] as List).cast<String>();
      final receivers = virtualFormats.map((format) {
        return _VirtualFileReceiver(
          readerManager: this,
          handle: handle,
          format: format,
        );
      }).toList(growable: false);
      return DataReaderItemInfo(
        handle,
        formats: (e['formats'] as List).cast<String>(),
        synthesizedFormats: (e['synthesizedFormats'] as List).cast<String>(),
        virtualReceivers: receivers,
        suggestedName: e['suggestedName'],
        synthesizedFromURIFormat: e['fileUriFormat'],
      );
    }).toList(growable: false);
    return res;
  }
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

class _VirtualFile extends VirtualFile {
  _VirtualFile({
    required this.readerManager,
    required this.handle,
    this.fileName,
    this.length,
  });

  @override
  void close() {
    readerManager.virtualFileClose(handle: handle);
  }

  @override
  Future<Uint8List> readNext() async {
    return (await readerManager.virtualFileRead(handle: handle)) ??
        Uint8List(0);
  }

  final ReaderManagerImpl readerManager;
  final int handle;

  @override
  final String? fileName;

  @override
  final int? length;
}

class _VirtualFileReceiver extends VirtualFileReceiver {
  _VirtualFileReceiver({
    required this.readerManager,
    required this.handle,
    required this.format,
  });

  @override
  (Future<VirtualFile>, ReadProgress) receiveVirtualFile() {
    return readerManager.virtualFileCreate(handle, format: format);
  }

  @override
  (Future<String>, ReadProgress) copyVirtualFile({
    required String targetFolder,
  }) {
    return readerManager.copyVirtualFile(
      handle,
      format: format,
      targetFolder: targetFolder,
    );
  }

  final ReaderManagerImpl readerManager;
  final DataReaderItemHandle handle;

  @override
  final String format;
}

class _CopyVirtualFileReceiver extends CopyVirtualFileReceiver {
  _CopyVirtualFileReceiver({
    required this.readerManager,
    required this.handle,
    required this.format,
  });

  @override
  (Future<String>, ReadProgress) copyVirtualFile({
    required String targetFolder,
  }) {
    return readerManager.copyVirtualFile(
      handle,
      format: format,
      targetFolder: targetFolder,
    );
  }

  final ReaderManagerImpl readerManager;
  final DataReaderItemHandle handle;

  @override
  final String format;
}
