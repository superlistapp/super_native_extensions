import 'dart:js_interop';

import 'package:web/web.dart' as web;

import '../clipboard_events.dart';
import '../data_provider.dart';
import '../reader.dart';
import '../reader_manager.dart';

import 'drop.dart';
import 'reader_manager.dart';

class _PasteEvent extends ClipboardReadEvent {
  _PasteEvent({
    required this.reader,
    required this.event,
  });

  final DataReader reader;
  final web.Event event;

  bool _defaultPrevented = false;

  @override
  DataReader getReader() {
    if (!_defaultPrevented) {
      _defaultPrevented = true;
      event.preventDefault();
    }
    return reader;
  }
}

class _WriteEvent extends ClipboardWriteEvent {
  _WriteEvent({required this.event});

  void _setData(String type, Object? data) {
    if (data is! String) {
      throw UnsupportedError('HTML Clipboard event only supports String data.');
    }
    event.clipboardData?.setData(type, data);
  }

  @override
  void write(List<DataProviderHandle> providers) {
    event.preventDefault();
    for (final provider in providers) {
      for (final repr in provider.provider.representations) {
        if (repr is DataRepresentationSimple) {
          _setData(repr.format, repr.data);
        } else if (repr is DataRepresentationLazy) {
          final data = repr.dataProvider();
          if (data is Future) {
            throw UnsupportedError(
                'HTML Clipboard event does not support asynchronous data.');
          }
          _setData(repr.format, data);
        }
      }
    }
  }

  final web.ClipboardEvent event;
}

class ClipboardEventsImpl extends ClipboardEvents {
  ClipboardEventsImpl() {
    web.window.addEventListener('paste', _onPaste.toJS);
    web.window.addEventListener('copy', _onCopy.toJS);
    web.window.addEventListener('cut', _onCut.toJS);
  }

  @override
  bool get supported => true;

  void _onPaste(web.Event event) {
    final clipboardEvent = event as web.ClipboardEvent;
    final itemList = clipboardEvent.clipboardData?.items;
    if (itemList == null) {
      return;
    }
    final translated =
        translateDataTransfer(clipboardEvent.clipboardData!, allowReader: true);
    final readerHandle = $DataReaderHandle(
      translated.map(
        (e) {
          return e.$2 as $DataReaderItemHandle;
        },
      ).toList(growable: false),
    );
    final reader = DataReader(handle: readerHandle as DataReaderHandle);
    final readEvent = _PasteEvent(reader: reader, event: event);
    for (final listener in _pasteEventListeners) {
      listener(readEvent);
    }
  }

  void _onCopy(web.Event event) {
    final clipboardEvent = event as web.ClipboardEvent;
    final writeEvent = _WriteEvent(event: clipboardEvent);
    for (final listener in _copyEventListeners) {
      listener(writeEvent);
    }
  }

  void _onCut(web.Event event) {
    final clipboardEvent = event as web.ClipboardEvent;
    final writeEvent = _WriteEvent(event: clipboardEvent);
    for (final listener in _cutEventListeners) {
      listener(writeEvent);
    }
  }

  final _pasteEventListeners = <void Function(ClipboardReadEvent reader)>[];
  final _copyEventListeners = <void Function(ClipboardWriteEvent reader)>[];
  final _cutEventListeners = <void Function(ClipboardWriteEvent reader)>[];

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
}
