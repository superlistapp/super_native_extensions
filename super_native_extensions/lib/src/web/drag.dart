import 'dart:html' as html;
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:super_native_extensions/src/web/shadow.dart';
import 'package:super_native_extensions/src/web/drag_driver.dart';

import '../api_model.dart';
import '../drag.dart';
import '../drag_internal.dart';
import '../util.dart';
import 'drop.dart';

class DragSessionImpl extends DragSession implements DragDriverDelegate {
  DragSessionImpl({required int pointer}) {
    DragDriver(pointer, this);
  }

  @override
  ValueListenable<DropOperation?> get dragCompleted => _dragCompleted;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);

  @override
  Listenable get dragStarted => _dragStarted;

  @override
  bool get dragging => _state != null;

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
  void end(Offset position) {
    _ended = true;
    _state?.end(position);
  }

  @override
  void update(Offset position) {
    if (!_ended) {
      _state?.update(position);
    }
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

class _SessionState implements DragDriverDelegate {
  final DragConfiguration configuration;
  final TargettedImageData image;
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
    final image = (await combineDragImage(configuration)).withShadow(14);
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
  }

  void _moveCanvas(Offset position) {
    canvas.style.left =
        '${image.rect.left + position.dx - originalPosition.dx}px';
    canvas.style.top =
        '${image.rect.top + position.dy - originalPosition.dy}px';
    canvas.style.width = '${image.rect.width}px';
    canvas.style.height = '${image.rect.height}px';
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

  void _cleanup() {
    DropContextImpl.instance?.localSessionDidEnd(configuration);
  }

  @override
  void cancel() {
    _removeCanvas(cancelled: true);
    dragCompleted.value = DropOperation.userCancelled;
    _cleanup();
  }

  @override
  void end(Offset position) async {
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
  void update(Offset position) {
    updatePosition(position);
  }
}

class DragContextImpl extends DragContext {
  @override
  Future<void> initialize() async {}

  @override
  DragSession newSession({int? pointer}) =>
      DragSessionImpl(pointer: pointer ?? -1);

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
