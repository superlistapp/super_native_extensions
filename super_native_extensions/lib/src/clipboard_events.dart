import 'data_provider.dart';
import 'native/clipboard_events.dart'
    if (dart.library.js_interop) 'web/clipboard_events.dart';
import 'reader.dart';

abstract class ClipboardReadEvent {
  /// Returns the clipboard reader. Once requested, this will prevent
  /// browser from performing default paste action, such as inserting
  /// text into input or content editable elements.
  DataReader getReader();
}

abstract class ClipboardWriteEvent {
  void write(List<DataProviderHandle> providers);
}

abstract class ClipboardEvents {
  static final ClipboardEvents instance = ClipboardEventsImpl();

  /// Returns whether paste event is supported on current platform. This is
  /// only supported on web.
  bool get supported;

  void registerPasteEventListener(void Function(ClipboardReadEvent) listener);

  void unregisterPasteEventListener(void Function(ClipboardReadEvent) listener);

  void registerCopyEventListener(void Function(ClipboardWriteEvent) listener);

  void unregisterCopyEventListener(void Function(ClipboardWriteEvent) listener);

  void registerCutEventListener(void Function(ClipboardWriteEvent) listener);

  void unregisterCutEventListener(void Function(ClipboardWriteEvent) listener);
}
