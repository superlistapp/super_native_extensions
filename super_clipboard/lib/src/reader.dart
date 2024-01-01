import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'system_clipboard.dart';
import 'format.dart';
import 'reader_internal.dart';
import 'standard_formats.dart';
import 'reader_model.dart';

abstract class DataReaderFile {
  /// Returns file name for the file, if available. File name at this
  /// point, if present, will be more reliable than the one provided
  /// by [DataReader.getSuggestedName];
  String? get fileName;

  /// Returns the file size if available.
  int? get fileSize;

  /// Returns the result of the data as stream. This can only be called once per
  /// value. Stream must be requested within the `onFile` callback.
  Stream<Uint8List> getStream();

  /// Closes the file. This only needs to be called manually when stream
  /// was requested through [getStream] but not consumed. Otherwise it is called
  /// automatically at the end of value callback or when stream is consumed.
  void close();

  /// Reads the rest of the data and returns it as a single chunk.
  Future<Uint8List> readAll();
}

typedef AsyncValueChanged<T> = FutureOr<void> Function(T value);

abstract class DataReader {
  /// Returns true value for data format is possibly available in this reader.
  ///
  /// Note that it is possible for [getValue] to return `null` even though
  /// [canProvide] returns yes, because in some cases this can not be fully
  /// determined from the format string, but only from the data itself.
  ///
  /// For example on some platforms file URI and regular URI have same type,
  /// so when receiving [Formats.fileUri] the decoder will have to fetch the value
  /// and will return null if URI is not a file uri.
  bool canProvide(DataFormat format) {
    return getFormats([format]).isNotEmpty;
  }

  @Deprecated('use canProvide instead')
  bool hasValue(DataFormat format) => canProvide(format);

  /// Returns subset of [allFormats] that this reader can provide,
  /// sorted according to priority set by source application.
  List<DataFormat> getFormats(List<DataFormat> allFormats);

  /// Loads the value for the given format.
  ///
  /// If no value for given format is available, `null` progress is returned
  /// and the [onValue] block will not be called.
  ///
  /// Getting the value is intentionally not exposed as async operation in order
  /// to prevent awaiting in contexts where it could block platform code (i.e.
  /// drop handle during drag and drop).
  ///
  /// When reading value form clipboard you can use the async variant in
  /// [ClipboardDataReader].
  ///
  /// Note that it is possible to receive a `null` value despite [canProvide]
  /// returning true. Sometimes the presence of value can not be determined
  /// just form the format string, but only from the data itself. For example
  /// file and regular URI have same type on some platforms, so when receiving
  /// [Formats.fileUri] the decoder will have to fetch the value and will return
  /// null if URI is not a file uri.
  ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<T?> onValue, {
    ValueChanged<Object>? onError,
  });

  /// Loads file for the given format.
  ///
  /// If no file for given format is available, `null` progress is returned and
  /// the [onFile] block will not be called.
  ///
  /// Returned progress tracks the progress from method invocation to receiving
  /// the file object. To track progress of reading the file you can use
  /// reported file size in [DataReaderFile] when you read the stream.
  ///
  /// On most platform the progress will be indeterminate followed by 1.0 at
  /// the end. On iOS the progress is bridged to underlying NSProgress object
  /// and should be more accurate and cancellable.
  ReadProgress? getFile(
    FileFormat? format,
    AsyncValueChanged<DataReaderFile> onFile, {
    ValueChanged<Object>? onError,
    bool allowVirtualFiles = true,
    bool synthesizeFilesFromURIs = true,
  });

  /// Returns whether value for given format is being synthesized. On Windows
  /// DIB images are accessible as PNG (converted on demand), same thing is
  /// done on macOS for TIFF images.
  ///
  /// On desktop platforms file URIs are also exposed as files with appropriate
  /// formats so they can be read through [DataReaderFile] API. For those
  /// [isSynthesized] will also return `true`.
  bool isSynthesized(DataFormat format);

  /// When `true`, data in this format is virtual. It means it might not be
  /// readily available and may be generated on demand. This is true for example
  /// when dropping images from iPhone (they will be downloaded after dropped).
  bool isVirtual(DataFormat format);

  /// Returns suggested file name for the contents (if available).
  /// This is the best guess that can be provided from reader. You may be able
  /// to get more accurate name after receiving the [DataReaderFile] through
  /// [getFile].
  Future<String?> getSuggestedName();

  /// Returns virtual file receiver for given format or `null` if virtual data
  /// for the format is not available. If format is not specified returns
  /// receiver for format with highest priority (if any).
  ///
  /// Usually it is not needed to call this method directly, as [getFile]
  /// will automatically call it if virtual data is available.
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
    FileFormat? format,
  });

  /// Returns list of platform specific format identifiers for this item.
  List<PlatformFormat> get platformFormats;

  /// If this reader is backed by raw DataReaderItem returns it.
  raw.DataReaderItem? get rawReader => null;

  /// Creates data reader from provided item info.
  static DataReader forItemInfo(raw.DataReaderItemInfo info) =>
      ItemDataReader.fromItemInfo(info);
}

abstract class ClipboardDataReader extends DataReader {
  /// Convenience method that exposes loading value as Future.
  ///
  /// Attempts to read value for given format. Will return `null` if the value
  /// is not available or the data is virtual (macOS and Windows).
  Future<T?> readValue<T extends Object>(ValueFormat<T> format);

  static ClipboardDataReader forItemInfo(raw.DataReaderItemInfo item) =>
      ItemDataReader.fromItemInfo(item);
}

/// Clipboard reader exposes contents of the clipboard.
class ClipboardReader extends ClipboardDataReader {
  ClipboardReader(this.items);

  /// Individual items of this clipboard reader.
  final List<ClipboardDataReader> items;

  @Deprecated('Use SystemClipboard.instance?.read() instead.')
  static Future<ClipboardReader> readClipboard() async {
    final clipboard = SystemClipboard.instance;
    if (clipboard == null) {
      throw UnsupportedError('Clipboard API is not available on this platform');
    }
    return clipboard.read();
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
  bool canProvide(DataFormat format) {
    return items.any((item) => item.canProvide(format));
  }

  @override
  ReadProgress? getValue<T extends Object>(
    ValueFormat<T> format,
    AsyncValueChanged<T?> onValue, {
    ValueChanged<Object>? onError,
  }) {
    final item =
        items.firstWhereOrNull((element) => element.canProvide(format));
    if (item != null) {
      return item.getValue(
        format,
        onValue,
        onError: onError,
      );
    } else {
      return null;
    }
  }

  @override
  ReadProgress? getFile(
    FileFormat? format,
    AsyncValueChanged<DataReaderFile> onFile, {
    ValueChanged<Object>? onError,
    bool allowVirtualFiles = true,
    bool synthesizeFilesFromURIs = true,
  }) {
    if (format == null) {
      return null;
    }
    final item =
        items.firstWhereOrNull((element) => element.canProvide(format));
    if (item != null) {
      return item.getFile(format, onFile,
          onError: onError,
          allowVirtualFiles: allowVirtualFiles,
          synthesizeFilesFromURIs: synthesizeFilesFromURIs);
    } else {
      return null;
    }
  }

  @override
  Future<T?> readValue<T extends Object>(ValueFormat<T> format) async {
    final item =
        items.firstWhereOrNull((element) => element.canProvide(format));
    return item?.readValue(format);
  }

  @override
  bool isSynthesized(DataFormat format) {
    return items.any((item) => item.isSynthesized(format));
  }

  @override
  bool isVirtual(DataFormat format) {
    return items.any((item) => item.isVirtual(format));
  }

  @override
  Future<VirtualFileReceiver?> getVirtualFileReceiver({
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
