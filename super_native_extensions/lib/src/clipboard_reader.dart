import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/raw_clipboard.dart';
import 'package:super_native_extensions/src/reader_manager.dart';

import 'context.dart';

class RawClipboardReader {
  /// Returns clipboard reader for current clipboard. Note that on some platforms
  /// the clipboard content for single reader will not change during the lifetime
  /// of the reader. On top of it the content is cached lazily.
  ///  If you need updated information create a new reader.
  Future<DataReader> newClipboardReader() async {
    final handle = await _channel.invokeMethod('newClipboardReader');
    return DataReader(handle: DataReaderHandle.deserialize(handle));
  }

  RawClipboardReader._();

  final _channel = NativeMethodChannel('ClipboardReader',
      context: superNativeExtensionsContext);

  static final instance = RawClipboardReader._();
}
