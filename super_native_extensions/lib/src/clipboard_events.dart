import 'data_provider.dart';
import 'native/clipboard_events.dart'
    if (dart.library.js) 'web/clipboard_events.dart';
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

  /// Registers a listener for paste event. This is only supported on web.
  /// It is a no-op on other platforms.
  ///
  /// The clipboard access will not display any any paste prompt UI,
  /// unlike accessing clipboard through [newClipboardReader].
  void registerPasteEventListener(void Function(ClipboardReadEvent) listener);

  /// Removes a listener for paste event. This is currently only supported on web.
  void unregisterPasteEventListener(void Function(ClipboardReadEvent) listener);

  void registerCopyEventListener(void Function(ClipboardWriteEvent) listener);

  void unregisterCopyEventListener(void Function(ClipboardWriteEvent) listener);

  void registerCutEventListener(void Function(ClipboardWriteEvent) listener);

  void unregisterCutEventListener(void Function(ClipboardWriteEvent) listener);
}
