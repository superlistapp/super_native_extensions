import '../clipboard_events.dart';

class ClipboardEventsImpl extends ClipboardEvents {
  @override
  bool get supported => false;

  @override
  void registerPasteEventListener(
      void Function(ClipboardReadEvent p1) listener) {}

  @override
  void unregisterPasteEventListener(
      void Function(ClipboardReadEvent p1) listener) {}

  @override
  void registerCopyEventListener(
      void Function(ClipboardWriteEvent p1) listener) {}

  @override
  void registerCutEventListener(
      void Function(ClipboardWriteEvent p1) listener) {}

  @override
  void unregisterCopyEventListener(
      void Function(ClipboardWriteEvent p1) listener) {}

  @override
  void unregisterCutEventListener(
      void Function(ClipboardWriteEvent p1) listener) {}
}
