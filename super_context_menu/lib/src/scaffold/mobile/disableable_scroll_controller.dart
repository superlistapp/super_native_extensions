import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

class DisableableScrollController extends ScrollController {
  DisableableScrollController(this.canScrollListenable);

  final ValueListenable<bool> canScrollListenable;

  @override
  ScrollPosition createScrollPosition(ScrollPhysics physics,
      ScrollContext context, ScrollPosition? oldPosition) {
    return _ScrollPositionWithSingleContext(
      canScrollListenable: canScrollListenable,
      physics: physics,
      context: context,
      initialPixels: initialScrollOffset,
      keepScrollOffset: keepScrollOffset,
      oldPosition: oldPosition,
      debugLabel: debugLabel,
    );
  }

  void detachListener() {
    for (final position in positions) {
      (position as _ScrollPositionWithSingleContext)._detachListener();
    }
  }
}

class _ScrollPositionWithSingleContext extends ScrollPositionWithSingleContext {
  _ScrollPositionWithSingleContext({
    required this.canScrollListenable,
    required super.physics,
    required super.context,
    super.initialPixels = 0.0,
    super.keepScrollOffset,
    super.oldPosition,
    super.debugLabel,
  }) {
    canScrollListenable.addListener(_updateCanScroll);
  }

  void _updateCanScroll() {
    if ((context as ScrollableState).mounted) {
      context.setCanDrag(_prevDrag && !_disableDrag);
    }
  }

  final ValueListenable<bool> canScrollListenable;

  @override
  void applyNewDimensions() {
    super.applyNewDimensions();
    _prevDrag = physics.shouldAcceptUserOffset(this);
    if (_disableDrag) {
      context.setCanDrag(false);
    }
  }

  void _detachListener() {
    canScrollListenable.removeListener(_updateCanScroll);
  }

  @override
  void dispose() {
    _detachListener();
    super.dispose();
  }

  bool get _disableDrag => !canScrollListenable.value;
  bool _prevDrag = false;
}
