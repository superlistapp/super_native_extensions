import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';

import 'mutex.dart';
import 'reader_manager.dart';

class DataReader {
  Future<List<DataReaderItem>> getItems() {
    return _mutex.protect(() async {
      _items ??= await RawReaderManager.instance.getItems(_handle);
      return _items!;
    });
  }

  DataReader({
    required DataReaderHandle handle,
  }) : _handle = handle;

  Future<void> dispose() => RawReaderManager.instance.dispose(_handle);

  final _mutex = Mutex();

  final DataReaderHandle _handle;
  List<DataReaderItem>? _items;
}

abstract class ReadProgress {
  /// Range is 0.0 to 1.0.
  /// Starts with null (indeterminate progress).
  /// Guaranteed to fire at least once on both completion or failure
  /// (with value of 1.0).
  ValueListenable<double?> get fraction;

  /// This may change over time, client must be prepared to handle that.
  ValueListenable<bool> get cancellable;

  void cancel();
}

typedef GetDataResult = DataResult<Object?>;

class DataResult<T> {
  DataResult(this.data, this.error);

  bool get isError => error != null;

  @override
  String toString() {
    if (error != null) {
      return error!.toString();
    } else if (data != null) {
      return data.toString();
    } else {
      return '<null>';
    }
  }

  final T data;
  final PlatformException? error;
}

class DataReaderItem {
  Future<List<String>> getAvailableFormats() {
    return _mutex.protect(() async {
      _availableFormats ??=
          await RawReaderManager.instance.getItemFormats(_handle);
      return _availableFormats!;
    });
  }

  ReadProgress getDataForFormat(
    String format, {
    required ValueChanged<GetDataResult> onData,
  }) {
    return RawReaderManager.instance
        .getItemData(_handle, format: format, onData: onData);
  }

  Future<VirtualFileReceiver?> getVirtualFileReceiver(
      {required String format}) async {
    if (await RawReaderManager.instance
        .canGetVirtualFile(_handle, format: format)) {
      return VirtualFileReceiver._(item: _handle, format: format);
    } else {
      return null;
    }
  }

  DataReaderItem({
    required DataReaderItemHandle handle,
  }) : _handle = handle;

  final DataReaderItemHandle _handle;

  final _mutex = Mutex();
  List<String>? _availableFormats;
}

class VirtualFileReceiver {
  VirtualFileReceiver._({
    required this.item,
    required this.format,
  });

  ReadProgress receiveVirtualFile({
    /// Target folder must be same for all files received in one session.
    required String targetFolder,
    required ValueChanged<DataResult<String?>> onResult,
  }) {
    return RawReaderManager.instance.getVirtualFile(item,
        format: format, targetFolder: targetFolder, onResult: onResult);
  }

  final DataReaderItemHandle item;
  final String format;
}
