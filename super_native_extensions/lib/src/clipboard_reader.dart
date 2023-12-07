import 'dart:async';

import 'native/clipboard_reader.dart'
    if (dart.library.js) 'web/clipboard_reader.dart';
import 'reader.dart';

abstract class PasteEvent {
  DataReader get reader;

  /// Prevents browser from performing default paste action, such as inserting
  /// text into input or content editable elements.
  void preventDefault();
}

abstract class ClipboardReader {
  static final ClipboardReader instance = ClipboardReaderImpl();

  /// Returns clipboard reader for current clipboard. Note that on some platforms
  /// the clipboard content for single reader will not change during the lifetime
  /// of the reader. On top of it the content is cached lazily.
  ///
  /// If you need updated information create a new reader.
  Future<DataReader> newClipboardReader();

  /// Returns whether paste event is supported on current platform. This is
  /// only supported on web.
  bool get supportsPasteEvent;

  /// Registers a listener for paste event. This is only supported on web.
  /// It is a no-op on other platforms.
  ///
  /// The clipboard access will not display any any paste prompt UI,
  /// unlike accessing clipboard through [newClipboardReader].
  void registerPasteEventListener(void Function(PasteEvent) listener);

  /// Removes a listener for paste event. This is currently only supported on web.
  void unregisterPasteEventListener(void Function(PasteEvent) listener);
}
