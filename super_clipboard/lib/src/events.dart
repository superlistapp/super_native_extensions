import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'reader.dart';
import 'writer.dart';
import 'writer_data_provider.dart';
import 'system_clipboard.dart';

/// Event dispatched during a browser paste action (only available on web).
/// Allows reading data from clipboard.
class ClipboardReadEvent {
  /// Returns the clipboard reader for paste event, which is not restricted nor requires user
  /// confirmation. This is the preferred way of reading clipboard data on web.
  ///
  /// Once requested, this will prevent browser from performing default paste action,
  /// such as inserting text into input or content editable elements.
  Future<ClipboardReader> getClipboardReader() async {
    final readerItems = await _event.getReader().getItems();
    final itemInfo = await raw.DataReaderItem.getItemInfo(readerItems);
    final items = itemInfo
        .map(
          (e) => ClipboardDataReader.forItemInfo(e),
        )
        .toList(growable: false);
    return ClipboardReader(items);
  }

  ClipboardReadEvent._({
    required raw.ClipboardReadEvent event,
  }) : _event = event;

  final raw.ClipboardReadEvent _event;
}

/// Event dispatched during copy and cut actions (only available on web).
/// Allows writing data to clipboard. However this is generally more limited than
/// [ClipboardWriter] and only allows writing text contents. It also does
/// not support providing data asynchronously.
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

  /// Returns clipboard events instance if available on current platform.
  /// This is only supported on web, on other platforms use [SystemClipboard.instance]
  /// to access the clipboard.
  static ClipboardEvents? get instance =>
      raw.ClipboardEvents.instance.supported ? _instance : null;

  static final _instance = ClipboardEvents._();

  /// Registers a listener for paste event (triggered through Ctrl/Cmd + V or browser menu action).
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

  /// Registers a listener for copy event (triggered through Ctrl/Cmd + C or browser menu action).
  ///
  /// The clipboard event can only be used to write text data and does not allow
  /// asynchronous data providers.
  void registerCopyEventListener(void Function(ClipboardWriteEvent) listener) {
    _copyEventListeners.add(listener);
  }

  /// Unregisters a listener for copy event previously registered with [registerCopyEventListener].
  void unregisterCopyEventListener(
      void Function(ClipboardWriteEvent) listener) {
    _copyEventListeners.remove(listener);
  }

  /// Registers a listener for cut event (triggered through Ctrl/Cmd + X or browser menu action).
  ///
  /// The clipboard event can only be used to write text data and does not allow
  /// asynchronous data providers.
  void registerCutEventListener(void Function(ClipboardWriteEvent) listener) {
    _cutEventListeners.add(listener);
  }

  /// Unregisters a listener for cut event previously registered with [registerCutEventListener].
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
