import 'reader.dart';
import 'writer.dart';
import 'writer_data_provider.dart';
import 'events.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

class SystemClipboard implements ClipboardWriter {
  /// Returns the shared clipboard instance if available on the current platform.
  /// Clipboard API is available on all platforms except Firefox, where it is
  /// disabled by default.
  /// If clipboard is not available, you can still use the [ClipboardEvents] API.
  static SystemClipboard? get instance {
    if (!raw.ClipboardReader.instance.available) {
      return null;
    }
    return _instance;
  }

  static final _instance = SystemClipboard._();

  /// Writes the content of the [items] to the clipboard.
  @override
  Future<void> write(Iterable<DataWriterItem> items) async {
    await items.withHandles((handles) async {
      await raw.ClipboardWriter.instance.write(handles);
    });
  }

  /// Reads clipboard contents. Note that on some platforms accessing clipboard may trigger
  /// a prompt for user to confirm clipboard access. This is the case on iOS and web.
  ///
  /// For web the preferred way to get clipboard contents is through
  /// [ClipboardEvents.registerPasteEventListener], which is triggered when user pastes something
  /// into the page and does not require any user confirmation.
  Future<ClipboardReader> read() async {
    final reader = await raw.ClipboardReader.instance.newClipboardReader();
    final readerItems = await reader.getItems();
    final itemInfo = await raw.DataReaderItem.getItemInfo(readerItems);
    final items = <ClipboardDataReader>[];
    for (final item in itemInfo) {
      items.add(ClipboardDataReader.forItemInfo(item));
    }
    return ClipboardReader(items);
  }

  SystemClipboard._();
}
