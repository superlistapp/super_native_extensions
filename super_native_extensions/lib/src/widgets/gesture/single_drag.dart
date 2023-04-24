import 'dart:async';

import 'package:flutter/gestures.dart';

class SingleDragUpdateDetails {
  SingleDragUpdateDetails({
    required this.globalPosition,
  });

  final Offset globalPosition;
}

class SingleDragEndDetails {
  final Velocity velocity;

  SingleDragEndDetails({
    required this.velocity,
  });
}

abstract class SingleDrag {
  void longPressRecognized();
  void update(SingleDragUpdateDetails details) {}
  void end(SingleDragEndDetails details) {}
  void cancel() {}
}

class SingleDragGestureRecognizer extends PanGestureRecognizer {
  SingleDragGestureRecognizer({
    super.debugOwner,
    super.supportedDevices,
  }) {
    onStart = _onStart;
    onUpdate = _onUpdate;
    onEnd = _onEnd;
    onCancel = _onLongPressCancel;
  }

  /// Called at the start of the gesture.
  SingleDrag? Function(Offset globalPosition)? onDragStart;

  /// Allows rejecting pointer events at specific position.
  bool Function(Offset globalPosition)? shouldAcceptTouchAtPosition;

  /// Returns last pointer that was accepted by this recognizer.
  int? get lastPointer => _lastPointer;

  int? _lastPointer;

  SingleDrag? _currentDrag;

  void _onStart(DragStartDetails details) {
    _currentDrag = onDragStart?.call(
      details.globalPosition,
    );
  }

  void _onUpdate(DragUpdateDetails details) {
    _currentDrag?.update(SingleDragUpdateDetails(
      globalPosition: details.globalPosition,
    ));
  }

  void _onEnd(DragEndDetails details) {
    // PanGestureRecognizer clobbers cancel / onEnd event, but we need to
    // distinguish between them because onCancel means drag started as expected
    // while onEnd means user released the pointer before drag started.
    if (_dispatchingCancel) {
      _currentDrag?.cancel();
    } else {
      _currentDrag?.end(SingleDragEndDetails(
        velocity: details.velocity,
      ));
    }
    _currentDrag = null;
  }

  void _onLongPressCancel() {
    _currentDrag?.cancel();
    _currentDrag = null;
  }

  @override
  void dispose() {
    _onLongPressCancel();
    super.dispose();
  }

  @override
  void acceptGesture(int pointer) {
    _lastPointer = pointer;
    super.acceptGesture(pointer);
  }

  @override
  bool isPointerAllowed(PointerEvent event) {
    if (!(shouldAcceptTouchAtPosition?.call(event.position) ?? true)) {
      return false;
    }
    return super.isPointerAllowed(event);
  }

  bool _dispatchingCancel = false;

  @override
  void handleEvent(PointerEvent event) {
    if (event is PointerCancelEvent) {
      try {
        _dispatchingCancel = true;
        super.handleEvent(event);
      } finally {
        _dispatchingCancel = false;
      }
    } else {
      super.handleEvent(event);
    }
  }
}

class SingleDragDelayedGestureRecognizer extends LongPressGestureRecognizer {
  SingleDragDelayedGestureRecognizer({
    required this.beginDuration,
    required super.duration,
    super.debugOwner,
    super.supportedDevices,
    super.postAcceptSlopTolerance,
  }) {
    assert(beginDuration < super.deadline!);
    onLongPressDown = _onLongPressDown;
    onLongPressStart = _onLongPressStart;
    onLongPressMoveUpdate = _onLongPressMoveUpdate;
    onLongPressEnd = _onLongPressEnd;
    onLongPressCancel = _onLongPressCancel;
  }

  /// Duration after which the lifting begins.
  final Duration beginDuration;

  /// Called at the start of the gesture.
  SingleDrag? Function(Offset globalPosition)? onDragStart;

  /// Allows rejecting pointer events at specific position.
  bool Function(Offset globalPosition)? shouldAcceptTouchAtPosition;

  /// Returns last pointer that was accepted by this recognizer.
  int? get lastPointer => _lastPointer;

  int? _lastPointer;

  SingleDrag? _currentDrag;

  Timer? _currentDragTimer;

  bool _recognized = false;

  void _onLongPressStart(LongPressStartDetails details) {
    assert(_currentDrag != null);
    assert(!_recognized);
    _recognized = true;
    _currentDrag?.longPressRecognized();
  }

  void _onLongPressMoveUpdate(LongPressMoveUpdateDetails details) {
    _currentDrag?.update(SingleDragUpdateDetails(
      globalPosition: details.globalPosition,
    ));
  }

  void _onLongPressEnd(LongPressEndDetails details) {
    _currentDrag?.end(SingleDragEndDetails(
      velocity: details.velocity,
    ));
    _currentDrag = null;
  }

  void _onLongPressDown(LongPressDownDetails details) {
    _recognized = false;
    assert(_currentDragTimer == null);
    assert(_currentDrag == null);
    _currentDragTimer = Timer(beginDuration, () {
      _currentDrag = onDragStart?.call(
        details.globalPosition,
      );
      _currentDragTimer = null;
    });
  }

  void _onLongPressCancel({bool rejected = false}) {
    if (!rejected && !_recognized) {
      // Canceled before recognized - simulate end gesture to animate lift back.
      _currentDrag?.end(SingleDragEndDetails(velocity: Velocity.zero));
      _currentDrag = null;
    }

    _currentDragTimer?.cancel();
    _currentDragTimer = null;

    _currentDrag?.cancel();
    _currentDrag = null;
  }

  @override
  void rejectGesture(int pointer) {
    // For some reason when vertical / horizontal drag start is recognized
    // onLongPressCancel is not invoked.
    _onLongPressCancel(rejected: true);
    super.rejectGesture(pointer);
  }

  @override
  void dispose() {
    _onLongPressCancel();
    super.dispose();
  }

  @override
  bool isPointerAllowed(PointerDownEvent event) {
    if (!(shouldAcceptTouchAtPosition?.call(event.position) ?? true)) {
      return false;
    }
    final res = super.isPointerAllowed(event);
    if (res) {
      _lastPointer = event.pointer;
    }
    return res;
  }
}
