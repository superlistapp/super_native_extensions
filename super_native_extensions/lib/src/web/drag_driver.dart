import 'dart:js_interop';
import 'dart:ui' as ui;

import 'package:flutter/gestures.dart';
import 'package:web/web.dart' as web;

abstract class DragDriverDelegate {
  void cancel();
  void update(ui.Offset position);
  void end(ui.Offset position);
}

class DragDriver {
  DragDriver({
    required this.pointer,
    required this.delegate,
    required this.devicePixelRatio,
  }) {
    web.document.addEventListener(
      'keydown',
      _keyDown = _onKeyDown.toJS,
      true.toJS,
    );
    // During drag all pointer events to Flutter need to be postponed
    // in order to be consistent with how drag&drop works on desktop platforms.
    // Flutter web registers mouse move listener on dom window. Since
    // there is no way to override listener itself, we override the
    // onPointerDataPacket callback instead.
    _previousPointerDataPacketCallback =
        ui.PlatformDispatcher.instance.onPointerDataPacket;
    ui.PlatformDispatcher.instance.onPointerDataPacket = _onPointerDataPacket;
  }

  final double devicePixelRatio;
  final int pointer;
  late ui.PointerDataPacketCallback? _previousPointerDataPacketCallback;
  late web.EventListener _keyDown;

  dynamic _onKeyDown(Object event) {
    final keyEvent = event as web.KeyboardEvent;
    if (keyEvent.key.toLowerCase() == 'escape') {
      cancel();
    }
  }

  void _cleanup() {
    web.document.removeEventListener('keydown', _keyDown, true.toJS);
    ui.PlatformDispatcher.instance.onPointerDataPacket =
        _previousPointerDataPacketCallback;
  }

  bool _didReleasePointer = false;

  void _onPointerDataPacket(ui.PointerDataPacket packet) {
    for (final data in packet.data) {
      _onPointerDataPacketInner(ui.PointerDataPacket(data: [data]));
    }
  }

  void _onPointerDataPacketInner(ui.PointerDataPacket packet) {
    // If this is not our packet pass it through.
    if (packet.data.any((element) =>
        element.pointerIdentifier != pointer ||
        element.signalKind == ui.PointerSignalKind.scroll)) {
      _previousPointerDataPacketCallback?.call(packet);
      return;
    }
    final data = packet.data.first;
    if (!_didReleasePointer) {
      if (data.change == ui.PointerChange.move) {
        // Synthesize pointer up event to pass to framework.
        _didReleasePointer = true;
        final newData = ui.PointerData(
          buttons: 0,
          pointerIdentifier: data.pointerIdentifier,
          device: data.device,
          change: ui.PointerChange.cancel,
          kind: data.kind,
          timeStamp: data.timeStamp,
          physicalX: data.physicalX,
          physicalY: data.physicalY,
          synthesized: true,
        );
        _previousPointerDataPacketCallback
            ?.call(ui.PointerDataPacket(data: [newData]));
      } else if (data.change == ui.PointerChange.up) {
        // This data already is pointer up event. There was no move.
        _previousPointerDataPacketCallback?.call(packet);
      }
      _didReleasePointer = true;
    }
    final offset = ui.Offset(
        data.physicalX / devicePixelRatio, data.physicalY / devicePixelRatio);
    delegate.update(offset);
    if (data.change == ui.PointerChange.up ||
        data.change == ui.PointerChange.cancel ||
        data.change == ui.PointerChange.remove) {
      delegate.end(offset);
      // Synthesize hover packet to properly update mouse regions.
      if (data.kind == PointerDeviceKind.mouse) {
        final newData = ui.PointerData(
          buttons: 0,
          device: data.device,
          pointerIdentifier: data.pointerIdentifier,
          change: ui.PointerChange.hover,
          kind: data.kind,
          timeStamp: data.timeStamp,
          physicalX: data.physicalX,
          physicalY: data.physicalY,
          synthesized: true,
        );
        _previousPointerDataPacketCallback
            ?.call(ui.PointerDataPacket(data: [newData]));
      }
      _cleanup();
    }
  }

  void cancel() {
    _cleanup();
    delegate.cancel();
  }

  final DragDriverDelegate delegate;
}
