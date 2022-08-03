import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'draggable_widget.dart';
import 'drag_configuration.dart';
import 'into_raw.dart';

class DragSession {
  DragSession._(raw.DragSession session) : _session = session;

  /// Fired when session dragging started.
  Listenable get dragStarted => _session.dragStarted;

  /// Fired on drag completion. The value will contain drop operation that the
  /// drag finished with.
  ValueListenable<raw.DropOperation?> get dragCompleted =>
      _session.dragCompleted;

  /// Updated when drag session moves. On mobile and web you will only
  /// get notified when moving over application Window.
  /// On desktop platforms the notification covers entire screen.
  ValueListenable<Offset?> get lastScreenLocation =>
      _session.lastScreenLocation;

  final raw.DragSession _session;
}

typedef LocationDraggableProvider = bool Function(Offset position);
typedef DragConfigurationProvider = Future<DragConfiguration?> Function(
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
    this.isLocationDraggable = _defaultIsLocationDraggable,
  });

  final Widget child;

  /// Returns drag configuration for the given offset and session.
  final DragConfigurationProvider dragConfiguration;

  /// Should return true if the offset is considered draggable.
  /// The offset is in global coordinates but restricted to area covered
  /// by the Widget.
  final LocationDraggableProvider isLocationDraggable;

  static bool _defaultIsLocationDraggable(Offset position) => true;

  @override
  Widget build(BuildContext context) {
    var child = this.child;
    if (defaultTargetPlatform == TargetPlatform.iOS && !kIsWeb) {
      // handled by delegate
    } else if (defaultTargetPlatform == TargetPlatform.android ||
        defaultTargetPlatform == TargetPlatform.iOS) {
      child = _MobileDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    } else {
      child = _DesktopDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    }
    return _BaseDragableRenderObject(
      onGetDragConfiguration: dragConfiguration,
      onIsLocationDraggable: isLocationDraggable,
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
    required this.onGetDragConfiguration,
    required this.onIsLocationDraggable,
  });

  final DragConfigurationProvider onGetDragConfiguration;
  final LocationDraggableProvider onIsLocationDraggable;

  @override
  RenderObject createRenderObject(BuildContext context) {
    _initializeIfNeeded();
    return _RenderBaseDraggable(
      devicePixelRatio: MediaQuery.of(context).devicePixelRatio,
      onGetDragConfiguration: onGetDragConfiguration,
      onIsLocationDraggable: onIsLocationDraggable,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject_) {
    final renderObject = renderObject_ as _RenderBaseDraggable;
    renderObject.devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    renderObject.onGetDragConfiguration = onGetDragConfiguration;
    renderObject.onIsLocationDraggable = onIsLocationDraggable;
  }
}

class _RenderBaseDraggable extends RenderProxyBoxWithHitTestBehavior {
  _RenderBaseDraggable({
    required this.devicePixelRatio,
    required this.onGetDragConfiguration,
    required this.onIsLocationDraggable,
  });

  double devicePixelRatio;
  DragConfigurationProvider onGetDragConfiguration;
  LocationDraggableProvider onIsLocationDraggable;
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
        final configuration = await target.onGetDragConfiguration(
          location,
          DragSession._(session),
        );
        return configuration?.intoRaw(target.devicePixelRatio);
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
        return target.onIsLocationDraggable(location);
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

  void _maybeStartDragWithSession(
    raw.DragContext context,
    Offset position,
    raw.DragSession session,
    double devicePixelRatio,
  ) async {
    final dragConfiguration =
        await this.dragConfiguration(position, DragSession._(session));
    if (dragConfiguration != null) {
      context.startDrag(
          session: session,
          configuration: await dragConfiguration.intoRaw(devicePixelRatio),
          position: position);
    }
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
        ImmediateMultiDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
                    ImmediateMultiDragGestureRecognizer>(
                () => ImmediateMultiDragGestureRecognizer(), (recognizer) {
          recognizer.onStart =
              (offset) => maybeStartDrag(offset, devicePixelRatio);
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
