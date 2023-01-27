import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'format.dart';
import 'reader_internal.dart';
import 'standard_formats.dart';
export 'package:super_native_extensions/raw_clipboard.dart'
    show VirtualFileReceiver, Pair, ReadProgress;

class DataReaderResult<T> {
  DataReaderResult({
    this.value,
    this.error,
  });

  final T? value;
  final Object? error;
}

abstract class DataReaderFile {
  /// Returns file name for the file, if available. File name at this
  /// point, if available, will be more reliable than the one provided
  /// by [DataReader.getSuggestedName];
  String? get fileName;

  /// Returns the file size if available.
  int? get fileSize;

  /// Closes the file. Generally this only needs to call this when stream
  /// was requested through [getStream] but not consumed. Otherwise it is called
  /// automatically at the end of value callback or when stream is consumed.
  void close();

  /// Returns the result of the data as stream. This can only be called once per
  /// value. Stream must be requested within the `onFile` callback.
  Stream<Uint8List> getStream();

  /// Reads the rest of the data and returns it as a single chunk.
  Future<Uint8List> readAll();
}

typedef AsyncValueChanged<T> = FutureOr<void> Function(T value);

abstract class DataReader {
  /// Returns true value for data format is possibly available in this reader.
  ///
  /// Note that it is expected for [getValue] to return `null` even though
  /// [hasValue] returns yes, because in some cases this can not be fully
  /// determined from the format string, but only from the data itself.
  ///
  /// For example on some platforms file URI and regular URI have same type,
  /// so when receiving [Formats.fileUri] the decoder will have to fetch the value
  /// and will return null if URI is not a file uri.
  bool hasValue(DataFormat format) {
    return getFormats([format]).isNotEmpty;
  }

  /// Returns subset of [allFormats] that this reader can provide,
  /// sorted according to priority.
  List<DataFormat> getFormats(List<DataFormat> allFormats);

  /// Loads the value for the given format.
  ///
  /// If no value for given format is available, `null` progress is returned and
  /// [onValue] is called immediately with `null` result.
  ///
  /// Getting the value is intentionally not exposed as async operation in order
  /// to prevent awaiting in contexts where it could block platform code (i.e.
  /// drop handle during drag and drop).
  ///
  /// When reading value form clipboard you can use the async variant in
  /// [ClipboardDataReader].
  raw.ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<DataReaderResult<T>> onValue,
  );

  /// Loads file for the given format.
  ///
  /// If no file for given format is available, `null` progress is returned and
  /// [onFile] is called immediately with `null` result.
  raw.ReadProgress? getFile(
    FileFormat format,
    AsyncValueChanged<DataReaderResult<DataReaderFile>> onFile, {
    bool allowVirtualFiles = true,
    bool synthetizeFilesFromURIs = true,
  });

  /// Returns whether value for given format is being synthetized. On Windows
  /// DIB images are accessible as PNG (converted on demand), same thing is
  /// done on macOS for TIFF images.
  bool isSynthetized(DataFormat format);

  /// When `true`, data in this format is virtual. It means it might not be
  /// readily available and may be generated on demand. This is true for example
  /// when dropping images from iPhone (they will be downloaded after dropped).
  bool isVirtual(DataFormat format);

  /// Returns suggested file name for the contents (if available).
  Future<String?> getSuggestedName();

  /// Returns virtual file receiver for given format or `null` if virtual data
  /// for the format is not available. If format is not specified returns
  /// receiver for format with highest priority (if any).
  ///
  /// Usually it is not needed to call this method directly, as [getValue]
  /// will automatically call it if virtual data is available.
  Future<raw.VirtualFileReceiver?> getVirtualFileReceiver({
    FileFormat? format,
  });

  /// Returns list of platform specific format identifiers for this item.
  List<PlatformFormat> get platformFormats;

  /// If this reader is backed by raw DataReaderItem returns it.
  raw.DataReaderItem? get rawReader => null;

  static Future<DataReader> forItem(raw.DataReaderItem item) async =>
      ItemDataReader.fromItem(item);
}

abstract class ClipboardDataReader extends DataReader {
  /// Convenience method that exposes loading value as Future.
  ///
  /// Attempts to read value for given format. Will return `null` if the value
  /// is not available or the data is virtual (macOS and Windows).
  Future<T?> readValue<T extends Object>(ValueFormat<T> format);

  static Future<ClipboardDataReader> forItem(raw.DataReaderItem item) async =>
      ItemDataReader.fromItem(item);
}

/// Clipboard reader exposes contents of the clipboard.
class ClipboardReader extends ClipboardDataReader {
  ClipboardReader._(this.items);

  /// Individual items of this clipboard reader.
  final List<ClipboardDataReader> items;

  static Future<ClipboardReader> readClipboard() async {
    final reader = await raw.ClipboardReader.instance.newClipboardReader();
    final readerItems = await reader.getItems();
    final items = <ClipboardDataReader>[];
    for (final item in readerItems) {
      items.add(await ClipboardDataReader.forItem(item));
    }
    return ClipboardReader._(items);
  }

  @override
  List<DataFormat> getFormats(List<DataFormat> allFormats) {
    final res = <DataFormat>[];
    for (final item in items) {
      final itemFormats = item.getFormats(allFormats);
      for (final format in itemFormats) {
        if (!res.contains(format)) {
          res.add(format);
        }
      }
    }
    return res;
  }

  @override
  bool hasValue(DataFormat format) {
    return items.any((item) => item.hasValue(format));
  }

  @override
  raw.ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<DataReaderResult<T>> onValue,
  ) {
    final item = items.firstWhereOrNull((element) => element.hasValue(format));
    if (item != null) {
      return item.getValue(
        format,
        onValue,
      );
    } else {
      onValue(DataReaderResult());
      return null;
    }
  }

  @override
  raw.ReadProgress? getFile(
    FileFormat format,
    AsyncValueChanged<DataReaderResult<DataReaderFile>> onFile, {
    bool allowVirtualFiles = true,
    bool synthetizeFilesFromURIs = true,
  }) {
    final item = items.firstWhereOrNull((element) => element.hasValue(format));
    if (item != null) {
      return item.getFile(format, onFile,
          allowVirtualFiles: allowVirtualFiles,
          synthetizeFilesFromURIs: synthetizeFilesFromURIs);
    } else {
      onFile(DataReaderResult());
      return null;
    }
  }

  @override
  Future<T?> readValue<T extends Object>(ValueFormat<T> format) async {
    final item = items.firstWhereOrNull((element) => element.hasValue(format));
    return item?.readValue(format);
  }

  @override
  bool isSynthetized(DataFormat format) {
    return items.any((item) => item.isSynthetized(format));
  }

  @override
  bool isVirtual(DataFormat format) {
    return items.any((item) => item.isVirtual(format));
  }

  @override
  Future<raw.VirtualFileReceiver?> getVirtualFileReceiver({
    FileFormat? format,
  }) async {
    for (final item in items) {
      final receiver = await item.getVirtualFileReceiver(format: format);
      if (receiver != null) {
        return receiver;
      }
    }
    return null;
  }

  @override
  Future<String?> getSuggestedName() async {
    for (final item in items) {
      final name = await item.getSuggestedName();
      if (name != null) {
        return name;
      }
    }
    return null;
  }

  @override
  List<PlatformFormat> get platformFormats {
    final res = <PlatformFormat>[];
    for (final item in items) {
      for (final format in item.platformFormats) {
        if (!res.contains(format)) {
          res.add(format);
        }
      }
    }
    return res;
  }
}
