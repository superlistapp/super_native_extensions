import 'dart:async';

import 'package:flutter/foundation.dart';

import 'mutex.dart';
import 'reader_manager.dart';

class DataReader {
  Future<List<DataReaderItem>> getItems() {
    return _mutex.protect(() async {
      _items ??= (await ReaderManager.instance.getItems(_handle))
          .map((handle) => DataReaderItem(handle: handle))
          .toList(growable: false);
      return _items!;
    });
  }

  DataReader({
    required DataReaderHandle handle,
  }) : _handle = handle;

  Future<void> dispose() => ReaderManager.instance.dispose(_handle);

  final _mutex = Mutex();

  final DataReaderHandle _handle;
  List<DataReaderItem>? _items;
}

abstract class ReadProgress {
  /// Range is 0.0 to 1.0.
  /// Starts with null (indeterminate progress).
  /// Guaranteed to fire at least once on either completion or failure
  /// (with value of 1.0).
  ValueListenable<double?> get fraction;

  /// This may change over time, client must be prepared to handle that.
  ValueListenable<bool> get cancellable;

  void cancel();
}

class Pair<T, U> {
  const Pair(this.first, this.second);

  final T first;
  final U second;
}

class DataReaderItem {
  Future<List<String>> getAvailableFormats() {
    return _mutex.protect(() async {
      _availableFormats ??=
          await ReaderManager.instance.getItemFormats(_handle);
      return _availableFormats!;
    });
  }

  Pair<Future<Object?>, ReadProgress> getDataForFormat(
    String format,
  ) {
    return ReaderManager.instance.getItemData(_handle, format: format);
  }

  Future<bool> isSynthetized(String format) {
    return ReaderManager.instance
        .itemFormatIsSynthetized(_handle, format: format);
  }

  /// When `true` the content can be received through [getVirtualFileReceiver].
  /// On macOS and Windows if [isVirtual] is `true` the content can only be
  /// received through [getVirtualFileReceiver] - [getDataForFormat] will return
  /// `null`.
  Future<bool> isVirtual(String format) {
    return ReaderManager.instance.canGetVirtualFile(_handle, format: format);
  }

  Future<String?> getSuggestedName() {
    return ReaderManager.instance.getItemSuggestedName(_handle);
  }

  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    required String format,
  }) async {
    if (await ReaderManager.instance
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

  Pair<Future<String?>, ReadProgress> receiveVirtualFile({
    /// Target folder must be same for all files received in one session.
    required String targetFolder,
  }) {
    return ReaderManager.instance
        .getVirtualFile(item, format: format, targetFolder: targetFolder);
  }

  final DataReaderItemHandle item;
  final String format;
}
