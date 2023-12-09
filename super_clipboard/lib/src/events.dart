import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'reader.dart';
import 'writer.dart';
import 'writer_data_provider.dart';

/// Paste event dispatched during a browser paste action (only available on web)
class ClipboardReadEvent {
  /// Returns the clipboard reader for paste event, which is not restricted nor requires user
  /// confirmation
  ///
  /// Once requested, this will prevent browser from performing default paste action,
  /// such as inserting text into input or content editable elements.
  Future<ClipboardReader> getClipboardReader() async {
    final readerItems = await _event.getReader().getItems();
    final items = await Future.wait(
      readerItems.map(
        (e) => ClipboardDataReader.forItem(e),
      ),
    );
    return ClipboardReader(items);
  }

  ClipboardReadEvent._({
    required raw.ClipboardReadEvent event,
  }) : _event = event;

  final raw.ClipboardReadEvent _event;
}

class ClipboardWriteEvent extends ClipboardWriter {
  ClipboardWriteEvent._({
    required raw.ClipboardWriteEvent event,
  }) : _event = event;

  final raw.ClipboardWriteEvent _event;

  @override
  Future<void> write(Iterable<DataWriterItem> items) async {
    items.withHandlesSync((handles) async {
      _event.write(handles);
    });
  }
}

class ClipboardEvents {
  ClipboardEvents._() {
    raw.ClipboardEvents.instance.registerPasteEventListener(_onPaste);
    raw.ClipboardEvents.instance.registerCopyEventListener(_onCopy);
    raw.ClipboardEvents.instance.registerCutEventListener(_onCut);
  }

  static final instance = ClipboardEvents._();

  /// Returns whether paste event is supported on current platform. This is
  /// only supported on web.
  bool get supported => raw.ClipboardEvents.instance.supported;

  /// Registers a listener for paste event (triggered through Ctrl/Cmd + V or browser menu action).
  /// This is only supported on web and is a no-op on other platforms.
  ///
  /// The clipboard access in the listener will not require any use conformation and allows
  /// accessing files, unlike [readClipboard] which is more limited on web.
  void registerPasteEventListener(void Function(ClipboardReadEvent) listener) {
    _pasteEventListeners.add(listener);
  }

  /// Unregisters a listener for paste event previously registered with [registerPasteEventListener].
  void unregisterPasteEventListener(
      void Function(ClipboardReadEvent) listener) {
    _pasteEventListeners.remove(listener);
  }

  void registerCopyEventListener(void Function(ClipboardWriteEvent) listener) {
    _copyEventListeners.add(listener);
  }

  void unregisterCopyEventListener(
      void Function(ClipboardWriteEvent) listener) {
    _copyEventListeners.remove(listener);
  }

  void registerCutEventListener(void Function(ClipboardWriteEvent) listener) {
    _cutEventListeners.add(listener);
  }

  void unregisterCutEventListener(void Function(ClipboardWriteEvent) listener) {
    _cutEventListeners.remove(listener);
  }

  void _onPaste(raw.ClipboardReadEvent event) {
    final pasteEvent = ClipboardReadEvent._(event: event);
    for (final listener in _pasteEventListeners) {
      listener(pasteEvent);
    }
  }

  void _onCopy(raw.ClipboardWriteEvent event) {
    final writeEvent = ClipboardWriteEvent._(event: event);
    for (final listener in _copyEventListeners) {
      listener(writeEvent);
    }
  }

  void _onCut(raw.ClipboardWriteEvent event) {
    final writeEvent = ClipboardWriteEvent._(event: event);
    for (final listener in _cutEventListeners) {
      listener(writeEvent);
    }
  }

  static final _pasteEventListeners =
      <void Function(ClipboardReadEvent event)>[];
  static final _copyEventListeners =
      <void Function(ClipboardWriteEvent event)>[];
  static final _cutEventListeners =
      <void Function(ClipboardWriteEvent event)>[];
}
