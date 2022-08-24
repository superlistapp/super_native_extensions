import 'native/clipboard_reader.dart'
    if (dart.library.js) 'web/clipboard_reader.dart';
import 'reader.dart';

abstract class ClipboardReader {
  static final ClipboardReader instance = ClipboardReaderImpl();

  /// Returns clipboard reader for current clipboard. Note that on some platforms
  /// the clipboard content for single reader will not change during the lifetime
  /// of the reader. On top of it the content is cached lazily.
  ///
  /// If you need updated information create a new reader.
  Future<DataReader> newClipboardReader();
}
