import 'dart:math';

import 'package:flutter/scheduler.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import '../gesture/single_drag.dart';

class DragState {
  DragState({
    required this.liftFactor,
    required this.menuFactor,
    required this.dragFactor,
    required this.menuOverdrag,
    required this.menuDragOffset,
    required this.globalPosition,
  });

  /// Transition from inactive (0.0) to fully lifted (1.0)
  final double liftFactor;

  /// Transition from lifted (lift factor will stay the same 0.0 - 1.0) to
  /// menu.
  final double menuFactor;

  /// Transition from lifted or menu (will stay the same, within 0.0 - 1.0)
  /// to drag.
  final double dragFactor;

  /// Menu offset while menu is being dragged but before transition to drag.
  final Offset menuOverdrag;

  /// Offset for draggable menus. Long menus need to be dragged first in order
  /// to be fully visible and scrollable.
  /// 0.0 means menu is fully scrolled away or not draggable, 1.0 means
  /// menu is fully visible (when draggable).
  final double menuDragOffset;

  /// Touch position in global coordinates
  final Offset globalPosition;

  copyWith({
    double? liftFactor,
    double? menuFactor,
    double? dragFactor,
    Offset? menuOverdrag,
    double? menuDragOffset,
    Offset? globalPosition,
  }) {
    return DragState(
      liftFactor: liftFactor ?? this.liftFactor,
      menuFactor: menuFactor ?? this.menuFactor,
      dragFactor: dragFactor ?? this.dragFactor,
      menuOverdrag: menuOverdrag ?? this.menuOverdrag,
      menuDragOffset: menuDragOffset ?? this.menuDragOffset,
      globalPosition: globalPosition ?? this.globalPosition,
    );
  }
}

abstract class DragDelegate {
  void updateState(DragState state);
  void menuDragEnded(double velocity);
  void cancel();
  void beginTransitonToMenu();
  void beginTransitionToDrag();
  void didNotFinishTransitionToDrag();
  void beginDrag(Offset globalPosition, int? pointer);
  void onTapUp(Offset globalPosition);

  bool canTransitionToDrag();
  bool isMenuOpened();
  bool hasMenu();
}

enum _State {
  none,
  lifting,
  lifted,
  liftedToMenu,
  menu,
  toDrag,
  waitingCancellationAfterDrag,
  cancelLift,
}

const _liftDuration = Duration(milliseconds: 500);

const _liftToMenuDuration = Duration(milliseconds: 250);

const _liftToDragDuration = Duration(milliseconds: 200);
const _menuToDragDuration = Duration(milliseconds: 200);

const _liftDragThreshold = 20.0;
const _menuDragThreshold = 80.0;

class DragInteractionDrag implements SingleDrag {
  DragInteractionDrag({
    required this.delegate,
    required this.pointer,
    required this.initialOffset,
    required this.menuDragExtent,
    required this.initialMenuDragOffset,
  }) : _currentPosition = initialOffset {
    assert(!initialMenuDragOffset.isNaN);
    _ticker = Ticker((elapsed) {
      currentTime = elapsed;
      _onTick();
    });
    _ticker.start();
  }

  final int pointer;

  final ValueGetter<double> menuDragExtent;
  final double initialMenuDragOffset;

  final Offset initialOffset;
  Offset _currentPosition;

  late Duration currentTime;
  final startTime = const Duration(seconds: 0);
  final DragDelegate delegate;
  late Ticker _ticker;

  var _menuOverdrag = Offset.zero;

  SingleDragEndDetails? _dragEndDetails;

  // Gesture has ended
  bool get _ended => _dragEndDetails != null;

  _State _state = _State.none;

  Duration? _begin;
  Duration? _end;

  /// Lift factor saved when transitioning from menu to drag.
  double? _liftFactor;

  /// Menu factor saved when transitioning from menu to drag.
  double? _menuFactor;

  double _menuDragOffset = 0;

  double _maxDistance = 0;

  final _creationTime = Stopwatch()..start();

  void _updateDelegate({
    double? liftFactor,
    double? menuFactor,
    double? dragFactor,
  }) {
    assert(liftFactor == null || liftFactor <= 1.0);
    assert(menuFactor == null || menuFactor <= 1.0);
    assert(dragFactor == null || dragFactor <= 1.0);
    delegate.updateState(
      DragState(
        liftFactor: liftFactor ?? 0.0,
        menuFactor: menuFactor ?? 0.0,
        dragFactor: dragFactor ?? 0.0,
        menuOverdrag: _menuOverdrag,
        menuDragOffset: _menuDragOffset,
        globalPosition: _currentPosition,
      ),
    );
  }

  void _onTick() {
    if (_disposed) {
      return;
    }

    if (_state == _State.none) {
      if (delegate.hasMenu() && delegate.isMenuOpened()) {
        _state = _State.menu;
        _menuFactor = 1.0;
        _liftFactor = 1.0;
      } else {
        _state = _State.lifting;
        _begin = currentTime;
        _end = currentTime + _liftDuration;
      }
    }

    switch (_state) {
      case _State.lifting:
        return _onTickLifting();
      case _State.lifted:
        return _onTickLifted();
      case _State.liftedToMenu:
        return _onTickLiftedToMenu();
      case _State.menu:
        return _onTickMenu();
      case _State.toDrag:
        return _onTickToDrag();
      case _State.cancelLift:
        return _onTickCancelLift();
      case _State.waitingCancellationAfterDrag:
        break;
      case _State.none:
        throw StateError('Invalid state: $_state');
    }
  }

  Offset _dragOffsetToActualOffset(double offset) {
    return Offset(0, offset * menuDragExtent());
  }

  double get _distance => (_currentPosition -
          initialOffset +
          _dragOffsetToActualOffset(_menuDragOffset) -
          _dragOffsetToActualOffset(initialMenuDragOffset))
      .distance;

  bool _longPressRecognized = false;

  @override
  void longPressRecognized() {
    HapticFeedback.selectionClick();
    _longPressRecognized = true;
  }

  void _onTickLifting() {
    assert(_begin != null);
    final factor = _computeFactor(_begin!, _end!);

    if (_distance > _liftDragThreshold && delegate.canTransitionToDrag()) {
      if (!_longPressRecognized) {
        _transitionToCancelLift();
      } else {
        _transitionToDrag();
      }
    } else {
      if (factor >= 1.0) {
        _transitionToLifted();
      } else {
        _updateDelegate(liftFactor: factor);
      }
    }
  }

  void _onTickLifted() {
    if (delegate.hasMenu()) {
      _transitionToMenu();
    } else {
      if (_distance > _liftDragThreshold && delegate.canTransitionToDrag()) {
        _transitionToDrag();
      } else {
        _updateDelegate(liftFactor: 1.0);
      }
    }
  }

  // Reverses the timepoint so that at now it interpolates at (1.0 - current
  // factor) and at now + newDuration it interpolates at 1.0
  Duration _reverseTimepoint(
    Duration originalEnd,
    Duration originalDuration,
    Duration newDuration,
  ) {
    if (originalEnd <= currentTime) {
      return currentTime;
    } else {
      final factor =
          _computeFactor(originalEnd - originalDuration, originalEnd);
      return currentTime - newDuration * (1.0 - factor);
    }
  }

  void _transitionToLifted() {
    assert(_begin != null);
    _begin = null;
    _state = _State.lifted;
    _liftFactor = 1.0;
  }

  void _transitionToCancelLift() {
    _begin = _reverseTimepoint(
      _end ?? currentTime,
      _liftDuration,
      _liftDuration,
    );
    _end = _begin! + _liftDuration;
    _state = _State.cancelLift;
  }

  bool _disposed = false;

  void dispose() {
    assert(!_disposed);
    _disposed = true;
    _ticker.stop();
    _ticker.dispose();
  }

  void _onTickCancelLift() {
    assert(_begin != null);
    final factor = _computeFactor(_begin!, _end!);
    if (factor >= 1.0) {
      dispose();
      delegate.cancel();
    } else {
      _updateDelegate(liftFactor: 1.0 - factor);
    }
  }

  void _transitionToDrag() {
    if (_begin != null) {
      final currentFactor = _computeFactor(_begin!, _end!).clamp(0.0, 1.0);
      if (_liftFactor == null) {
        _liftFactor = currentFactor;
        // ignore: prefer_conditional_assignment
      } else if (_menuFactor == null) {
        _menuFactor = currentFactor;
      }
    }
    _begin = currentTime;
    if (_state == _State.menu) {
      _end = currentTime + _menuToDragDuration;
    } else {
      _end = currentTime + _liftToDragDuration;
    }
    if (_state == _State.menu) {
      HapticFeedback.selectionClick();
    }
    _state = _State.toDrag;
    delegate.beginTransitionToDrag();
  }

  void _transitionToMenu() {
    assert(_state == _State.lifted || _state == _State.lifting);
    delegate.beginTransitonToMenu();
    if (_state == _State.lifting) {
      _liftFactor = _computeFactor(_begin!, _end!);
    }
    _state = _State.liftedToMenu;
    _begin = currentTime;
    _end = currentTime + _liftToMenuDuration;
    _maxDistance = double.infinity;
  }

  void _onTickToDrag() {
    final dragFactor = _computeFactor(_begin!, _end!);
    if (dragFactor >= 1.0) {
      if (_ended) {
        delegate.cancel();
        dispose();
      } else {
        _updateDelegate(
          liftFactor: _liftFactor,
          menuFactor: _menuFactor,
          dragFactor: 1.0,
        );
        delegate.beginDrag(_currentPosition, pointer);
        _state = _State.waitingCancellationAfterDrag;
      }
    } else {
      _updateDelegate(
        liftFactor: _liftFactor,
        menuFactor: _menuFactor,
        dragFactor: dragFactor,
      );
    }
  }

  void _updateMenuOverdrag() {
    var menuDelta = _currentPosition -
        initialOffset -
        _dragOffsetToActualOffset(initialMenuDragOffset);

    if (menuDelta.dy < 0) {
      final menuDragExtent = this.menuDragExtent();
      final menuDragOffsetPixels = min(-menuDelta.dy, menuDragExtent);
      menuDelta = Offset(menuDelta.dx, menuDelta.dy + menuDragOffsetPixels);
      _menuDragOffset =
          menuDragExtent > 0 ? menuDragOffsetPixels / menuDragExtent : 0;
    } else {
      _menuDragOffset = 0;
    }
    _menuOverdrag = menuDelta;
  }

  void _onTickLiftedToMenu() {
    assert(_begin != null && _end != null);

    _updateMenuOverdrag();

    final menuFactor = _computeFactor(_begin!, _end!);
    double liftFactor = _liftFactor! + (1.0 - _liftFactor!) * menuFactor;

    if (menuFactor >= 1.0) {
      _menuFactor = 1.0;
      _liftFactor = 1.0;
      _state = _State.menu;
    } else {
      if (!_ended &&
          _distance > _liftDragThreshold &&
          delegate.canTransitionToDrag()) {
        _transitionToDrag();
      } else {
        _updateDelegate(
          liftFactor: liftFactor,
          menuFactor: menuFactor,
        );
      }
    }
  }

  void _onTickMenu() {
    _updateMenuOverdrag();
    _updateDelegate(
      menuFactor: 1.0,
      liftFactor: 1.0,
    );
    if (_ended) {
      delegate.menuDragEnded(_dragEndDetails?.velocity.pixelsPerSecond.dy ?? 0);
      dispose();
    } else if (_distance > _menuDragThreshold &&
        delegate.canTransitionToDrag()) {
      _transitionToDrag();
    }
  }

  double _computeFactor(Duration beginTime, Duration endTime) {
    final now = currentTime;
    final elapsed = now - beginTime;
    final duration = endTime - beginTime;
    final factor = elapsed.inMilliseconds / duration.inMilliseconds;
    return factor;
  }

  void _ensureDisposed() async {
    await Future.delayed(const Duration(milliseconds: 1000));

    /// This means we have a bug in the state machine.
    assert(_disposed, 'DragStateMachine was not disposed.');
    if (!_disposed) {
      dispose();
      delegate.cancel();
    }
  }

  @override
  void cancel() {
    if (_disposed) {
      return;
    }
    _ensureDisposed();

    if (_state == _State.toDrag) {
      delegate.didNotFinishTransitionToDrag();
    }
    dispose();
    if (_state != _State.waitingCancellationAfterDrag) {
      delegate.cancel();
    }
  }

  @override
  void end(SingleDragEndDetails details) {
    if (_disposed) {
      return;
    }
    _ensureDisposed();

    _dragEndDetails = details;

    // Detect tap up even when menu is opened. This is done here rather than
    // separate recognizer because we don't want the tap recognizer to comppete
    // with drag (which would cause drag slop delay).
    if (_maxDistance < 20 &&
        _state == _State.menu &&
        _creationTime.elapsed < const Duration(milliseconds: 350)) {
      dispose();
      delegate.onTapUp(_currentPosition);
      return;
    }

    switch (_state) {
      case _State.lifting:
        if (_longPressRecognized && delegate.hasMenu()) {
          _transitionToMenu();
        } else {
          _transitionToCancelLift();
        }
        break;
      case _State.lifted:
        _transitionToCancelLift();
        break;
      case _State.toDrag:
        delegate.didNotFinishTransitionToDrag();
        break;
      case _State.liftedToMenu:
      case _State.menu:
      case _State.cancelLift:
        // nothing to do here, let the animation finish
        break;
      case _State.waitingCancellationAfterDrag:
        // Starting drag should result in gesture being cancelled.
        // If the gesture ends instead it means user lifted finger before
        // drag started.
        delegate.didNotFinishTransitionToDrag();
        delegate.cancel();
        dispose();
        break;
      case _State.none:
        delegate.cancel();
        // Nothing has started yet
        dispose();
    }
  }

  @override
  void update(SingleDragUpdateDetails details) {
    if (_disposed) {
      return;
    }
    _currentPosition = details.globalPosition;
    _maxDistance =
        max(_maxDistance, (_currentPosition - initialOffset).distance);
  }
}
