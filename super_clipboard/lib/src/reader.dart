import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'format.dart';
import 'reader_internal.dart';
import 'standard_formats.dart';
export 'package:super_native_extensions/raw_clipboard.dart'
    show VirtualFileReceiver, Pair, ReadProgress;

class DataReaderValue<T extends Object> {
  DataReaderValue({this.value, this.error});

  final T? value;
  final Object? error;
}

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
  /// [onValue] is caleld immediately with `null` result.
  ///
  /// Getting the value is intentionally not exposed as async operation in order
  /// to prevent awaiting in contexts where it could block platform code (i.e.
  /// drop handle during drag and drop).
  ///
  /// When reading value form clipboard you can use the async variant in
  /// [ClipboardDataReader].
  raw.ReadProgress? getValue<T extends Object>(
    DataFormat<T> format,
    ValueChanged<DataReaderValue<T>> onValue,
  );

  /// Returns whether value for given format is being synthetized. On Windows
  /// DIB images are accessible as PNG (converted on demand), same thing is
  /// done on macOS for TIFF images.
  bool isSynthetized(DataFormat format);

  /// When `true`, data in this format is virtual. It means it might not be
  /// readily available and may need to be retrieved through
  /// [getVirtualFileReceiver] instead of [readValue]. This is the case on macOS
  /// and Windows. On iOS virtual data can be received through both [readValue]
  /// and [getVirtualFileReceiver].
  bool isVirtual(DataFormat format);

  /// Returns suggested file name for the contents (if available).
  Future<String?> getSuggestedName();

  /// Returns virtual file receiver for given format or `null` if virtual data
  /// for the format is not available. If format is not specified returns
  /// receiver for format with highest priority (if any).
  Future<raw.VirtualFileReceiver?> getVirtualFileReceiver({
    VirtualFileFormat? format,
  });

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
  Future<T?> readValue<T extends Object>(DataFormat<T> format);

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
      DataFormat<T> format, ValueChanged<DataReaderValue<T>> onValue) {
    final item = items.firstWhereOrNull((element) => element.hasValue(format));
    if (item != null) {
      return item.getValue(format, onValue);
    } else {
      onValue(DataReaderValue(value: null));
      return null;
    }
  }

  @override
  Future<T?> readValue<T extends Object>(DataFormat<T> format) async {
    final item = items.firstWhereOrNull((element) => element.hasValue(format));
    return item?.readValue(format);
  }

  @override
  bool isSynthetized(DataFormat<Object> format) {
    return items.any((item) => item.isSynthetized(format));
  }

  @override
  bool isVirtual(DataFormat<Object> format) {
    return items.any((item) => item.isVirtual(format));
  }

  @override
  Future<raw.VirtualFileReceiver?> getVirtualFileReceiver({
    VirtualFileFormat? format,
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
}
