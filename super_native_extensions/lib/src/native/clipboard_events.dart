import 'dart:async';
import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import '../clipboard_events.dart';
import '../clipboard_reader.dart';
import '../clipboard_writer.dart';
import '../data_provider.dart';
import '../reader.dart';
import 'context.dart';

class _ClipboardWriteEvent extends ClipboardWriteEvent {
  final _completers = <Completer>[];

  @override
  void write(Object token, List<DataProviderHandle> providers) async {
    final completer = token as Completer;
    await ClipboardWriter.instance.write(providers);
    completer.complete();
  }

  @override
  Object beginWrite() {
    final completer = Completer();
    _completers.add(completer);
    return completer;
  }

  @override
  bool get isSynchronous => false;
}

class _ClipboardReadEvent extends ClipboardReadEvent {
  _ClipboardReadEvent(this.reader);

  final DataReader reader;
  bool didGetReader = false;

  @override
  DataReader getReader() {
    didGetReader = true;
    return reader;
  }
}

class ClipboardEventsImpl extends ClipboardEvents {
  ClipboardEventsImpl() {
    if (Platform.environment.containsKey('FLUTTER_TEST')) {
      return;
    }
    _channel.setMethodCallHandler(_onMethodCall);
    _channel.invokeMethod('newClipboardEventsManager');
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'copy') {
      final writeEvent = _ClipboardWriteEvent();
      for (final listener in _copyEventListeners) {
        listener(writeEvent);
      }
      if (writeEvent._completers.isNotEmpty) {
        await Future.wait(writeEvent._completers.map((e) => e.future));
        return true;
      } else {
        return false;
      }
    } else if (call.method == 'cut') {
      final writeEvent = _ClipboardWriteEvent();
      for (final listener in _cutEventListeners) {
        listener(writeEvent);
      }
      if (writeEvent._completers.isNotEmpty) {
        await Future.wait(writeEvent._completers.map((e) => e.future));
        return true;
      } else {
        return false;
      }
    } else if (call.method == 'paste') {
      final reader = await ClipboardReader.instance.newClipboardReader();
      final writeEvent = _ClipboardReadEvent(reader);
      for (final listener in _pasteEventListeners) {
        listener(writeEvent);
      }
      return writeEvent.didGetReader;
    } else if (call.method == 'selectAll') {
      bool handled = false;
      for (final listener in _textEventListeners) {
        handled |= listener(TextEvent.selectAll);
      }
      return handled;
    }
  }

  @override
  bool get supported => defaultTargetPlatform == TargetPlatform.iOS;

  final _pasteEventListeners = <void Function(ClipboardReadEvent reader)>[];
  final _copyEventListeners = <void Function(ClipboardWriteEvent reader)>[];
  final _cutEventListeners = <void Function(ClipboardWriteEvent reader)>[];
  final _textEventListeners = <bool Function(TextEvent)>[];

  @override
  void registerPasteEventListener(
      void Function(ClipboardReadEvent p1) listener) {
    _pasteEventListeners.add(listener);
  }

  @override
  void unregisterPasteEventListener(
      void Function(ClipboardReadEvent p1) listener) {
    _pasteEventListeners.remove(listener);
  }

  @override
  void registerCopyEventListener(
      void Function(ClipboardWriteEvent p1) listener) {
    _copyEventListeners.add(listener);
  }

  @override
  void unregisterCopyEventListener(
      void Function(ClipboardWriteEvent p1) listener) {
    _copyEventListeners.remove(listener);
  }

  @override
  void registerCutEventListener(
      void Function(ClipboardWriteEvent p1) listener) {
    _cutEventListeners.add(listener);
  }

  @override
  void unregisterCutEventListener(
      void Function(ClipboardWriteEvent p1) listener) {
    _cutEventListeners.remove(listener);
  }

  @override
  void registerTextEventListener(bool Function(TextEvent) listener) {
    _textEventListeners.add(listener);
  }

  @override
  void unregisterTextEventListener(bool Function(TextEvent) listener) {
    _textEventListeners.remove(listener);
  }

  final _channel = NativeMethodChannel('ClipboardEventManager',
      context: superNativeExtensionsContext);
}
