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

  static Future<String?> formatForFileUri(Uri uri) =>
      ReaderManager.instance.formatForFileUri(uri);

  DataReader({
    required DataReaderHandle handle,
  }) : _handle = handle;

  Future<void> dispose() => ReaderManager.instance.dispose(_handle);

  final _mutex = Mutex();

  final DataReaderHandle _handle;
  List<DataReaderItem>? _items;
}

/// Progress of a read operation.
abstract class ReadProgress {
  /// Range is 0.0 to 1.0.
  /// Starts with null (indeterminate progress).
  /// Guaranteed to fire at least once on either completion or failure
  /// (with value of 1.0).
  ValueListenable<double?> get fraction;

  /// This may change over time, client must be prepared to handle that.
  ValueListenable<bool> get cancellable;

  /// Cancels the read operation. Does nothing if already cancelled or not
  /// cancellable.
  void cancel();
}

class DataReaderItemInfo {
  DataReaderItemInfo(
    this._handle, {
    required this.formats,
    required this.synthesizedFormats,
    required this.virtualReceivers,
    required this.suggestedName,
    required this.synthesizedFromURIFormat,
  });

  DataReaderItem get item => DataReaderItem(handle: _handle);
  final List<String> formats;
  final List<String> synthesizedFormats;
  final List<VirtualFileReceiver> virtualReceivers;
  final String? suggestedName;
  final String? synthesizedFromURIFormat;
  final DataReaderItemHandle _handle;
}

class DataReaderItem {
  Future<List<String>> getAvailableFormats() {
    return _mutex.protect(() async {
      _availableFormats ??=
          await ReaderManager.instance.getItemFormats(_handle);
      return _availableFormats!;
    });
  }

  (Future<Object?>, ReadProgress) getDataForFormat(
    String format,
  ) {
    return ReaderManager.instance.getItemData(_handle, format: format);
  }

  static Future<List<DataReaderItemInfo>> getItemInfo(
      Iterable<DataReaderItem> items) {
    return ReaderManager.instance.getItemInfo(items.map((e) => e._handle));
  }

  Future<bool> isSynthesized(String format) {
    return ReaderManager.instance
        .itemFormatIsSynthesized(_handle, format: format);
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
    return ReaderManager.instance.createVirtualFileReceiver(
      _handle,
      format: format,
    );
  }

  @override
  bool operator ==(Object other) {
    return other is DataReaderItem && other._handle == _handle;
  }

  @override
  int get hashCode => _handle.hashCode;

  DataReaderItem({
    required DataReaderItemHandle handle,
  }) : _handle = handle;

  final DataReaderItemHandle _handle;

  final _mutex = Mutex();
  List<String>? _availableFormats;
}

abstract class VirtualFile {
  /// Returns the file name or `null` if not available.
  String? get fileName;

  /// Returns total length if available.
  int? get length;

  /// Reads next chunk of the data. Returns empty list when all data has been read.
  Future<Uint8List> readNext();

  /// Closes the virtual file.
  void close();

  /// Creates virtual file from file at specific file URI.
  /// Not supported on web.
  static VirtualFile fromFileUri(Uri uri) {
    return ReaderManager.instance.createVirtualFileFromUri(uri);
  }
}

abstract class VirtualFileReceiver {
  String get format;

  /// Receives the virtual file.
  (Future<VirtualFile>, ReadProgress) receiveVirtualFile();

  /// Copies the virtual file to specific folder and returns the path.
  /// Not available on web.
  (Future<String>, ReadProgress) copyVirtualFile({
    required String targetFolder,
  });
}
