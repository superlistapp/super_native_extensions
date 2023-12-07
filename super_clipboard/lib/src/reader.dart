import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

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

/// Paste event dispatched during a browser paste action (only available on web)
class PasteEvent {
  /// Returns the clipboard reader for paste event, which is not restricted nor requires user
  /// confirmation
  ///
  /// Once requested, this will prevent
  /// browser from performing default paste action, such as inserting
  /// text into input or content editable elements.
  Future<ClipboardReader> getClipboardReader() async {
    final readerItems = await _event.getReader().getItems();
    final items = await Future.wait(
      readerItems.map(
        (e) => ClipboardDataReader.forItem(e),
      ),
    );
    return ClipboardReader._(items);
  }

  PasteEvent._({
    required raw.PasteEvent event,
  }) : _event = event;

  final raw.PasteEvent _event;
}

/// Clipboard reader exposes contents of the clipboard.
class ClipboardReader extends ClipboardDataReader {
  ClipboardReader._(this.items);

  /// Individual items of this clipboard reader.
  final List<ClipboardDataReader> items;

  /// Reads clipboard contents. Note that on some platforms accessing clipboard may trigger
  /// a prompt for user to confirm clipboard access. This is the case on iOS and web.
  ///
  /// For web the preferred way to get clipboard contents is through [registerPasteEventListener],
  /// which is triggered when user pastes something into the page and does not require any
  /// user confirmation.
  static Future<ClipboardReader> readClipboard() async {
    final reader = await raw.ClipboardReader.instance.newClipboardReader();
    final readerItems = await reader.getItems();
    final items = <ClipboardDataReader>[];
    for (final item in readerItems) {
      items.add(await ClipboardDataReader.forItem(item));
    }
    return ClipboardReader._(items);
  }

  /// Returns whether paste event is supported on current platform. This is
  /// only supported on web.
  static bool get supportsPasteEvent =>
      raw.ClipboardReader.instance.supportsPasteEvent;

  /// Registers a listener for paste event (triggered through Ctrl/Cmd + V or browser menu action).
  /// This is only supported on web and is a no-op on other platforms.
  ///
  /// The clipboard access in the listener will not require any use conformation and allows
  /// accessing files, unlike [readClipboard] which is more limited on web.
  static void registerPasteEventListener(
    void Function(PasteEvent event) listener,
  ) {
    _pasteEventListeners.add(listener);
    if (!_pasteEventRegistered) {
      _pasteEventRegistered = true;
      raw.ClipboardReader.instance.registerPasteEventListener((event) async {
        final pasteEvent = PasteEvent._(event: event);
        for (final listener in _pasteEventListeners) {
          listener(pasteEvent);
        }
      });
    }
  }

  /// Unregisters a listener for paste event previously registered with [registerPasteEventListener].
  static void unregisterPasteEventListener(
    void Function(PasteEvent event) listener,
  ) {
    _pasteEventListeners.remove(listener);
  }

  static final _pasteEventListeners = <void Function(PasteEvent event)>[];

  static bool _pasteEventRegistered = false;

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
