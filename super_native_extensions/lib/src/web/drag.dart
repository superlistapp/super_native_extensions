import 'dart:html' as html;
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter_web_plugins/flutter_web_plugins.dart';
import 'package:super_native_extensions/raw_drag_drop.dart';
import 'package:super_native_extensions/src/util.dart';

import '../drag_internal.dart';
import 'drop.dart';

class DragSessionImpl extends DragSession implements Drag {
  @override
  ValueListenable<DropOperation?> get dragCompleted => _dragCompleted;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);

  @override
  Listenable get dragStarted => _dragStarted;

  final _dragStarted = SimpleNotifier();

  @override
  Future<List<Object?>?> getLocalData() async {
    return _state?.getLocalData();
  }

  @override
  ValueListenable<Offset?> get lastScreenLocation => _lastScreenLocation;

  final _lastScreenLocation = ValueNotifier<Offset?>(null);

  @override
  void cancel() {
    _ended = true;
    _state?.cancel();
  }

  @override
  void end(DragEndDetails details) {
    _ended = true;
    _state?.end(details);
  }

  @override
  void update(DragUpdateDetails details) {
    _state?.update(details);
  }

  bool _ended = false;

  Future<void> init(
      DragConfiguration configuration, Offset originalPosition) async {
    _state = await _SessionState.create(
      configuration: configuration,
      originalPosition: originalPosition,
      lastScreenLocation: _lastScreenLocation,
      dragCompleted: _dragCompleted,
    );
    if (_ended) {
      _state?.cancel();
    }
    _dragStarted.notify();
    _dragCompleted.addListener(() {
      _state = null;
    });
  }

  _SessionState? _state;
}

class _SessionState implements Drag {
  final DragConfiguration configuration;
  final DragImage image;
  final Offset originalPosition;
  final html.CanvasElement canvas;
  final ValueNotifier<Offset?> lastScreenLocation;
  final ValueNotifier<DropOperation?> dragCompleted;

  static Future<_SessionState> create({
    required DragConfiguration configuration,
    required Offset originalPosition,
    required ValueNotifier<Offset?> lastScreenLocation,
    required ValueNotifier<DropOperation?> dragCompleted,
  }) async {
    final image = await combineDragImage(configuration);
    final canvas = html.document.createElement('canvas') as html.CanvasElement;
    canvas.width = image.imageData.width;
    canvas.height = image.imageData.height;
    final ctx = canvas.getContext('2d') as html.CanvasRenderingContext2D;
    final imageData =
        ctx.createImageData(image.imageData.width, image.imageData.height);
    imageData.data.setAll(0, image.imageData.data);
    ctx.putImageData(imageData, 0, 0);
    originalPosition = originalPosition;
    html.document.body?.children.add(canvas);
    canvas.style.position = 'fixed';
    canvas.style.pointerEvents = 'none';
    canvas.style.boxShadow = '0px 0px 14px #00000080';
    return _SessionState(
      configuration: configuration,
      image: image,
      originalPosition: originalPosition,
      canvas: canvas,
      lastScreenLocation: lastScreenLocation,
      dragCompleted: dragCompleted,
    );
  }

  _SessionState({
    required this.configuration,
    required this.image,
    required this.originalPosition,
    required this.canvas,
    required this.lastScreenLocation,
    required this.dragCompleted,
  }) {
    updatePosition(originalPosition);
    html.document.addEventListener('keydown', onKeyDown = _onKeyDown, true);
  }

  late EventListener onKeyDown;

  void _moveCanvas(Offset position) {
    canvas.style.left =
        '${image.sourceRect.left + position.dx - originalPosition.dx}px';
    canvas.style.top =
        '${image.sourceRect.top + position.dy - originalPosition.dy}px';
    canvas.style.width = '${image.sourceRect.width}px';
    canvas.style.height = '${image.sourceRect.height}px';
  }

  void updatePosition(Offset position) async {
    _moveCanvas(position);
    lastScreenLocation.value = position;

    _lastOperation = await DropContextImpl.instance
            ?.localSessionDidMove(configuration, position) ??
        DropOperation.none;
  }

  DropOperation _lastOperation = DropOperation.none;

  List<Object?> getLocalData() {
    return configuration.items.map((e) => e.localData).toList(growable: false);
  }

  dynamic _onKeyDown(Object event) {
    final keyEvent = event as html.KeyboardEvent;
    if (keyEvent.key?.toLowerCase() == 'escape') {
      cancel();
    }
  }

  void _cleanup() {
    DropContextImpl.instance?.localSessionDidEnd(configuration);
    html.document.removeEventListener('keydown', onKeyDown, true);
  }

  @override
  void cancel() {
    _removeCanvas(cancelled: true);
    dragCompleted.value = DropOperation.userCancelled;
    _cleanup();
  }

  @override
  void end(DragEndDetails details) async {
    final location = lastScreenLocation.value;
    if (_lastOperation != DropOperation.none && location != null) {
      await DropContextImpl.instance
          ?.localSessionDrop(configuration, location, _lastOperation);
    }
    dragCompleted.value = _lastOperation;
    _removeCanvas(
      cancelled: _lastOperation == DropOperation.none ||
          _lastOperation == DropOperation.userCancelled ||
          _lastOperation == DropOperation.forbidden,
    );
    _cleanup();
  }

  void _removeCanvas({
    required bool cancelled,
  }) {
    if (cancelled) {
      double movementDuration;
      double distance =
          ((lastScreenLocation.value ?? originalPosition) - originalPosition)
              .distance;
      if (distance == 0) {
        movementDuration = 0;
      } else if (distance < 50) {
        movementDuration = 0.2;
      } else {
        movementDuration = 0.4;
      }
      canvas.style.transitionProperty = 'left, top';
      canvas.style.transitionDuration = '${movementDuration}s';
      _moveCanvas(originalPosition);
      Future.delayed(Duration(milliseconds: (movementDuration * 1000).round()),
          () {
        canvas.style.transitionProperty = 'opacity';
        canvas.style.transitionDuration = '0.2s';
        canvas.style.opacity = '0';
        Future.delayed(const Duration(milliseconds: 200), () {
          canvas.remove();
        });
      });
    } else {
      canvas.style.transitionProperty = 'opacity';
      canvas.style.transitionDuration = '0.2s';
      canvas.style.opacity = '0';
      Future.delayed(const Duration(milliseconds: 200), () {
        canvas.remove();
      });
    }
  }

  @override
  void update(DragUpdateDetails details) {
    updatePosition(details.globalPosition);
  }
}

class DragContextImpl extends DragContext {
  @override
  Future<void> initialize() async {}

  @override
  DragSession newSession() => DragSessionImpl();

  @override
  Future<void> startDrag({
    required DragSession session,
    required DragConfiguration configuration,
    required Offset position,
  }) async {
    final session_ = session as DragSessionImpl;
    await session_.init(configuration, position);
  }
}
