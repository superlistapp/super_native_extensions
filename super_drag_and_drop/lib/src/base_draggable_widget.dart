import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';

import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'draggable_widget.dart';
import 'drag_configuration.dart';
import 'into_raw.dart';

typedef LocationIsDraggable = bool Function(Offset position);
typedef DragConfigurationProvider = Future<DragConfiguration?> Function(
    Offset position, DragSession session);
typedef AdditionalItemsProvider = Future<List<DragItem>?> Function(
    Offset position, DragSession session);

/// This is the most basic draggable widget. It gives you complete control
/// over creating of the drag session.
///
/// In most cases you will want to use [DraggableWidget] instead.
class BaseDraggableWidget extends StatelessWidget {
  const BaseDraggableWidget({
    super.key,
    required this.child,
    required this.dragConfiguration,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    this.isLocationDraggable = _defaultIsLocationDraggable,
    this.additionalItems = _defaultAdditionalItems,
  });

  final Widget child;

  final HitTestBehavior hitTestBehavior;

  /// Returns drag configuration for the given offset and session.
  final DragConfigurationProvider dragConfiguration;

  /// Should return true if the offset is considered draggable.
  /// The offset is in global coordinates but restricted to area covered
  /// by the Widget.
  final LocationIsDraggable isLocationDraggable;

  /// On iOS this method is called when user taps draggable widget
  /// during existing drag sessions. It can be used to provide additional
  /// dragging item for current session.
  final AdditionalItemsProvider additionalItems;

  static Future<List<DragItem>?> _defaultAdditionalItems(
      Offset position, DragSession session) async {
    return null;
  }

  static bool _defaultIsLocationDraggable(Offset position) => true;

  @override
  Widget build(BuildContext context) {
    var child = this.child;
    if (defaultTargetPlatform == TargetPlatform.iOS && !kIsWeb) {
      // on iOS the drag detector is not used to start drag (dragging is driven
      // from iOS UI interaction). The delayed recognizer is needed because
      // otherwise the scroll activity disables user interaction too early
      // and the hit test fails.
      child = _DummyDragDetector(child: child);
    } else if (defaultTargetPlatform == TargetPlatform.android ||
        defaultTargetPlatform == TargetPlatform.iOS) {
      child = _MobileDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    } else {
      child = _DesktopDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    }
    return _BaseDragableRenderObject(
      hitTestBehavior: hitTestBehavior,
      getDragConfiguration: dragConfiguration,
      isLocationDraggable: isLocationDraggable,
      additionalItems: additionalItems,
      child: child,
    );
  }
}

//
// Implementation
//

class _BaseDragableRenderObject extends SingleChildRenderObjectWidget {
  const _BaseDragableRenderObject({
    required super.child,
    required this.hitTestBehavior,
    required this.getDragConfiguration,
    required this.isLocationDraggable,
    required this.additionalItems,
  });

  final HitTestBehavior hitTestBehavior;
  final DragConfigurationProvider getDragConfiguration;
  final LocationIsDraggable isLocationDraggable;
  final AdditionalItemsProvider additionalItems;

  @override
  RenderObject createRenderObject(BuildContext context) {
    _initializeIfNeeded();
    return _RenderBaseDraggable(
      behavior: hitTestBehavior,
      devicePixelRatio: MediaQuery.of(context).devicePixelRatio,
      getDragConfiguration: getDragConfiguration,
      isLocationDraggable: isLocationDraggable,
      additionalItems: additionalItems,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject_) {
    final renderObject = renderObject_ as _RenderBaseDraggable;
    renderObject.behavior = hitTestBehavior;
    renderObject.devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    renderObject.getDragConfiguration = getDragConfiguration;
    renderObject.isLocationDraggable = isLocationDraggable;
    renderObject.additionalItems = additionalItems;
  }
}

class _RenderBaseDraggable extends RenderProxyBoxWithHitTestBehavior {
  _RenderBaseDraggable({
    required super.behavior,
    required this.devicePixelRatio,
    required this.getDragConfiguration,
    required this.isLocationDraggable,
    required this.additionalItems,
  });

  double devicePixelRatio;
  DragConfigurationProvider getDragConfiguration;
  LocationIsDraggable isLocationDraggable;
  AdditionalItemsProvider additionalItems;
}

//

class _DragContextDelegate implements raw.DragContextDelegate {
  @override
  Future<raw.DragConfiguration?> getConfigurationForDragRequest({
    required Offset location,
    required raw.DragSession session,
  }) async {
    final hitTest = HitTestResult();
    GestureBinding.instance.hitTest(hitTest, location);
    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderBaseDraggable) {
        final configuration = await target.getDragConfiguration(
          location,
          session,
        );
        return configuration?.intoRaw(target.devicePixelRatio);
      }
    }
    return null;
  }

  @override
  Future<List<raw.DragItem>?> getAdditionalItemsForLocation(
      {required Offset location, required raw.DragSession session}) async {
    final hitTest = HitTestResult();
    GestureBinding.instance.hitTest(hitTest, location);
    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderBaseDraggable) {
        final additionalItems = await target.additionalItems(
          location,
          session,
        );
        return additionalItems?.intoRaw(target.devicePixelRatio);
      }
    }
    return null;
  }

  @override
  bool isLocationDraggable(Offset location) {
    final hitTest = HitTestResult();
    GestureBinding.instance.hitTest(hitTest, location);
    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderBaseDraggable) {
        return target.isLocationDraggable(location);
      }
    }
    return false;
  }
}

abstract class _DragDetector extends StatelessWidget {
  final Widget child;
  final DragConfigurationProvider dragConfiguration;

  const _DragDetector({
    required this.dragConfiguration,
    required this.child,
  });

  Drag? maybeStartDrag(Offset position, double devicePixelRatio) {
    final dragContext = _dragContext;
    if (dragContext != null) {
      final session = dragContext.newSession();
      _maybeStartDragWithSession(
          dragContext, position, session, devicePixelRatio);
      return session is Drag ? session as Drag : null;
    } else {
      return null;
    }
  }

  void onDraggingStarted() {}

  void _maybeStartDragWithSession(
    raw.DragContext context,
    Offset position,
    raw.DragSession session,
    double devicePixelRatio,
  ) async {
    final dragConfiguration = await this.dragConfiguration(position, session);
    if (dragConfiguration != null) {
      context.startDrag(
          session: session,
          configuration: await dragConfiguration.intoRaw(devicePixelRatio),
          position: position);
      onDraggingStarted();
    }
  }
}

class _ImmediateMultiDragGestureRecognizer
    extends ImmediateMultiDragGestureRecognizer {
  @override
  bool isPointerAllowed(PointerDownEvent event) {
    if (event.kind == PointerDeviceKind.mouse &&
        event.buttons != kPrimaryMouseButton) {
      return false;
    }
    return super.isPointerAllowed(event);
  }
}

class _DesktopDragDetector extends _DragDetector {
  const _DesktopDragDetector({
    required super.dragConfiguration,
    required super.child,
  });

  @override
  Widget build(BuildContext context) {
    final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    return RawGestureDetector(
      gestures: {
        _ImmediateMultiDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
                    _ImmediateMultiDragGestureRecognizer>(
                () => _ImmediateMultiDragGestureRecognizer(), (recognizer) {
          recognizer.onStart =
              (offset) => maybeStartDrag(offset, devicePixelRatio);
        })
      },
      child: child,
    );
  }
}

class _DummyDragDetector extends StatelessWidget {
  const _DummyDragDetector({
    required this.child,
  });

  final Widget child;

  @override
  Widget build(BuildContext context) {
    return RawGestureDetector(
      gestures: {
        DelayedMultiDragGestureRecognizer: GestureRecognizerFactoryWithHandlers<
                DelayedMultiDragGestureRecognizer>(
            () => DelayedMultiDragGestureRecognizer(), (recognizer) {
          recognizer.onStart = (offset) {
            return null;
          };
        })
      },
      child: child,
    );
  }
}

class _MobileDragDetector extends _DragDetector {
  const _MobileDragDetector({
    required super.dragConfiguration,
    required super.child,
  });

  @override
  Widget build(BuildContext context) {
    final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    return RawGestureDetector(
      gestures: {
        DelayedMultiDragGestureRecognizer: GestureRecognizerFactoryWithHandlers<
                DelayedMultiDragGestureRecognizer>(
            () => DelayedMultiDragGestureRecognizer(), (recognizer) {
          recognizer.onStart =
              (offset) => maybeStartDrag(offset, devicePixelRatio);
        })
      },
      child: child,
    );
  }

  @override
  void onDraggingStarted() {
    HapticFeedback.mediumImpact();
  }
}

bool _initialized = false;
raw.DragContext? _dragContext;

Future<void> _initializeIfNeeded() async {
  if (!_initialized) {
    _initialized = true;
    _dragContext = await raw.DragContext.instance();
    _dragContext!.delegate = _DragContextDelegate();
    // needed on some platforms (i.e. Android for drop end notifications)
    await raw.DropContext.instance();
  }
}
