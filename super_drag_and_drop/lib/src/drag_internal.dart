import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'base_draggable_widget.dart';
import 'into_raw.dart';

class BaseDraggableRenderWidget extends SingleChildRenderObjectWidget {
  const BaseDraggableRenderWidget({
    super.key,
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
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as _RenderBaseDraggable;
    renderObject_.behavior = hitTestBehavior;
    renderObject_.devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    renderObject_.getDragConfiguration = getDragConfiguration;
    renderObject_.isLocationDraggable = isLocationDraggable;
    renderObject_.additionalItems = additionalItems;
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
      if (target is _RenderBaseDraggable &&
          target.isLocationDraggable(location)) {
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
    super.key,
    required this.dragConfiguration,
    required this.child,
  });

  Drag? maybeStartDrag(
      int? pointer, Offset position_, double devicePixelRatio) {
    final position = Offset(
        (position_.dx * devicePixelRatio).roundToDouble() / devicePixelRatio,
        (position_.dy * devicePixelRatio).roundToDouble() / devicePixelRatio);
    final dragContext = _dragContext;
    if (dragContext != null) {
      final session = dragContext.newSession(pointer: pointer);
      if (pointer != null) {
        // Hide hover during dragging. The delay is here because there may
        // be some move events received until system drag starts)
        Future.delayed(const Duration(milliseconds: 50), () {
          if (session.dragging) {
            final event = PointerRemovedEvent(
                pointer: pointer, kind: PointerDeviceKind.mouse);
            RendererBinding.instance.mouseTracker
                .updateWithEvent(event, () => HitTestResult());
          }
        });
      }
      _maybeStartDragWithSession(
          dragContext, position, session, devicePixelRatio);
      return null;
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
    final dragConfiguration = await this.dragConfiguration(position, session);
    if (dragConfiguration != null) {
      context.startDrag(
          session: session,
          configuration: await dragConfiguration.intoRaw(devicePixelRatio),
          position: position);
    }
  }
}

class _ImmediateMultiDragGestureRecognizer
    extends ImmediateMultiDragGestureRecognizer {
  int? lastPointer;

  final LocationIsDraggable isLocationDraggable;

  _ImmediateMultiDragGestureRecognizer({
    required this.isLocationDraggable,
  });

  @override
  void acceptGesture(int pointer) {
    lastPointer = pointer;
    super.acceptGesture(pointer);
  }

  @override
  bool isPointerAllowed(PointerDownEvent event) {
    if (event.kind == PointerDeviceKind.mouse &&
        event.buttons != kPrimaryMouseButton) {
      return false;
    }
    if (!isLocationDraggable(event.position)) {
      return false;
    }
    return super.isPointerAllowed(event);
  }
}

class DesktopDragDetector extends _DragDetector {
  const DesktopDragDetector({
    super.key,
    required super.dragConfiguration,
    required this.isLocationDraggable,
    required super.child,
  });

  final LocationIsDraggable isLocationDraggable;

  @override
  Widget build(BuildContext context) {
    final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
    return RawGestureDetector(
      gestures: {
        _ImmediateMultiDragGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
                    _ImmediateMultiDragGestureRecognizer>(
                () => _ImmediateMultiDragGestureRecognizer(
                    isLocationDraggable: isLocationDraggable), (recognizer) {
          recognizer.onStart = (offset) =>
              maybeStartDrag(recognizer.lastPointer, offset, devicePixelRatio);
        })
      },
      child: child,
    );
  }
}

class DummyDragDetector extends StatelessWidget {
  const DummyDragDetector({
    super.key,
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

class MobileDragDetector extends _DragDetector {
  const MobileDragDetector({
    super.key,
    required super.dragConfiguration,
    required this.isLocationDraggable,
    required super.child,
  });

  final LocationIsDraggable isLocationDraggable;

  @override
  Widget build(BuildContext context) {
    return RawGestureDetector(
      gestures: {
        raw.SingleDragDelayedGestureRecognizer:
            GestureRecognizerFactoryWithHandlers<
                    raw.SingleDragDelayedGestureRecognizer>(
                () => raw.SingleDragDelayedGestureRecognizer(
                      beginDuration: const Duration(milliseconds: 150),
                      duration: const Duration(milliseconds: 300),
                    ), (recognizer) {
          recognizer.shouldAcceptTouchAtPosition = isLocationDraggable;
          recognizer.onDragStart = (globalPosition) {
            return _longPressHandler?.dragGestureForPosition(
              context: context,
              position: globalPosition,
              pointer: recognizer.lastPointer!,
            );
          };
        }),
      },
      child: child,
    );
  }
}

bool _initialized = false;
raw.DragContext? _dragContext;
raw.LongPressHandler? _longPressHandler;

void _initializeIfNeeded() async {
  if (!_initialized) {
    _initialized = true;
    _dragContext = await raw.DragContext.instance();
    _dragContext!.delegate = _DragContextDelegate();
    _longPressHandler = await raw.LongPressHandler.create();
    // needed on some platforms (i.e. Android for drop end notifications)
    await raw.DropContext.instance();
  }
}
