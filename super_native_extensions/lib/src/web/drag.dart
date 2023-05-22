import 'dart:html' as html;
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import '../drag.dart';
import '../drop.dart';
import '../widget_snapshot/widget_snapshot.dart';
import 'drag_overlay.dart';
import 'drop.dart';
import 'drag_driver.dart';

class DragSessionImpl extends DragSession implements DragDriverDelegate {
  DragSessionImpl({required this.pointer});

  final int pointer;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);

  final _dragging = ValueNotifier<bool>(false);

  @override
  ValueListenable<DropOperation?> get dragCompleted => _dragCompleted;

  @override
  ValueNotifier<bool> get dragging => _dragging;

  @override
  Future<List<Object?>?> getLocalData() async {
    return _state?.getLocalData();
  }

  @override
  ValueListenable<Offset?> get lastScreenLocation => _lastScreenLocation;

  final _lastScreenLocation = ValueNotifier<Offset?>(null);

  @override
  void cancel() {
    if (!_ended) {
      _ended = true;
      _state?.cancel();
    }
  }

  @override
  void end(Offset position) {
    if (!_ended) {
      _ended = true;
      _state?.end(position);
    }
  }

  @override
  void update(Offset position) {
    if (!_ended) {
      _state?.update(position);
    }
  }

  bool _ended = false;

  void init(
    BuildContext buildContext,
    DragConfiguration configuration,
    Offset originalPosition,
    TargetedWidgetSnapshot? combinedDragImage,
  ) {
    DragDriver(
      pointer: pointer,
      devicePixelRatio: MediaQuery.of(buildContext).devicePixelRatio,
      delegate: this,
    );
    _state = _SessionState(
      buildContext: buildContext,
      configuration: configuration,
      originalPosition: originalPosition,
      lastScreenLocation: _lastScreenLocation,
      dragCompleted: _dragCompleted,
      combinedDragImage: combinedDragImage,
    );
    _dragging.value = true;
    _dragCompleted.addListener(() {
      _dragging.value = false;
      _state = null;
      Future.microtask(() {
        _dragCompleted.dispose();
        _dragging.dispose();
        _lastScreenLocation.dispose();
        for (final item in configuration.items) {
          item.dataProvider.dispose();
        }
      });
    });
    if (_ended) {
      _state?.cancel();
    }
  }

  _SessionState? _state;
}

class _SessionState implements DragDriverDelegate {
  final DragConfiguration configuration;
  final Offset originalPosition;
  final ValueNotifier<Offset?> lastScreenLocation;
  final ValueNotifier<DropOperation?> dragCompleted;

  final dragOverlayKey = GlobalKey<DragOverlayState>();
  late OverlayEntry overlayEntry;

  _SessionState({
    required BuildContext buildContext,
    TargetedWidgetSnapshot? combinedDragImage,
    required this.configuration,
    required this.originalPosition,
    required this.lastScreenLocation,
    required this.dragCompleted,
  }) {
    final overlay = Overlay.of(buildContext);
    overlayEntry = OverlayEntry(
      builder: (context) {
        if (combinedDragImage != null) {
          return DragOverlayMobile(
            key: dragOverlayKey,
            snapshot: combinedDragImage,
            initialPosition: originalPosition,
          );
        } else {
          return DragOverlayDesktop(
            key: dragOverlayKey,
            initialPosition: originalPosition,
            snapshots:
                configuration.items.map((e) => e.image).toList(growable: false),
          );
        }
      },
    );
    overlay.insert(overlayEntry);

    updatePosition(originalPosition);
  }

  void updatePosition(Offset position) async {
    dragOverlayKey.currentState?.updatePosition(position);
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
    _removeCanvas(
        cancelled: true,
        onCompleted: () {
          dragCompleted.value = DropOperation.userCancelled;
          _cleanup();
        });
  }

  @override
  void end(Offset position) async {
    final location = lastScreenLocation.value;
    if (_lastOperation != DropOperation.none && location != null) {
      await DropContextImpl.instance
          ?.localSessionDrop(configuration, location, _lastOperation);
    }
    _removeCanvas(
      cancelled: _lastOperation == DropOperation.none ||
          _lastOperation == DropOperation.userCancelled ||
          _lastOperation == DropOperation.forbidden,
      onCompleted: () {
        dragCompleted.value = _lastOperation;
        _cleanup();
      },
    );
  }

  bool _removed = false;

  void _removeCanvas({
    required bool cancelled,
    required VoidCallback onCompleted,
  }) {
    assert(!_removed);
    _removed = true;
    void completion() {
      overlayEntry.remove();
      onCompleted();
    }

    if (cancelled) {
      int movementDuration;
      double distance =
          ((lastScreenLocation.value ?? originalPosition) - originalPosition)
              .distance;
      if (distance == 0) {
        movementDuration = 0;
      } else if (distance < 50) {
        movementDuration = 200;
      } else {
        movementDuration = 400;
      }
      dragOverlayKey.currentState?.animateHome(
          Duration(
            milliseconds: movementDuration,
          ),
          completion);
    } else {
      completion();
    }
  }

  @override
  void update(Offset position) {
    updatePosition(position);
  }
}

class DragContextImpl extends DragContext {
  static bool get isTouchDevice => html.window.navigator.maxTouchPoints != 0;

  @override
  Future<void> initialize() async {
    // Long press draggable requires disabling context menu.
    if (html.window.navigator.maxTouchPoints != 0) {
      html.document.addEventListener('contextmenu', (event) {
        final offset_ = (event as html.MouseEvent).offset;
        final offset = ui.Offset(offset_.x.toDouble(), offset_.y.toDouble());
        final draggable = delegate?.isLocationDraggable(offset) ?? false;
        if (draggable) {
          event.preventDefault();
        }
      });
    }
  }

  @override
  DragSession newSession({int? pointer}) =>
      DragSessionImpl(pointer: pointer ?? -1);

  @override
  Future<void> startDrag({
    required BuildContext buildContext,
    required DragSession session,
    required DragConfiguration configuration,
    required Offset position,
    TargetedWidgetSnapshot? combinedDragImage,
  }) async {
    final session_ = session as DragSessionImpl;
    session_.init(
      buildContext,
      configuration,
      position,
      combinedDragImage,
    );
  }
}
