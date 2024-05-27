import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
import 'package:super_native_extensions/raw_menu.dart';

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
    // TODO(knopp): Resolve when we can provide viewId from native side
    // ignore: deprecated_member_use
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
    // TODO(knopp): Resolve when we can provide viewId from native side
    // ignore: deprecated_member_use
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
    // TODO(knopp): Resolve when we can provide viewId from native side
    // ignore: deprecated_member_use
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

class _Drag implements Drag {
  bool _ended = false;

  @override
  void cancel() {
    _ended = true;
  }

  @override
  void end(DragEndDetails details) {
    _ended = true;
  }

  @override
  void update(DragUpdateDetails details) {}
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
    BuildContext buildContext,
    int? pointer,
    Offset position_,
    double devicePixelRatio,
  ) {
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
          if (session.dragging.value) {
            final event = PointerRemovedEvent(
                pointer: pointer, kind: PointerDeviceKind.mouse);

            RendererBinding.instance.mouseTracker
                .updateWithEvent(event, HitTestResult());
          }
        });
      }
      final drag = _Drag();
      _maybeStartDragWithSession(
        dragContext,
        buildContext,
        position,
        session,
        devicePixelRatio,
        drag,
      );
      return drag;
    } else {
      return null;
    }
  }

  void _maybeStartDragWithSession(
    raw.DragContext context,
    BuildContext buildContext,
    Offset position,
    raw.DragSession session,
    double devicePixelRatio,
    _Drag drag,
  ) async {
    final dragConfiguration = await this.dragConfiguration(position, session);
    // User ended the drag gesture before the data is available.
    if (drag._ended) {
      _dragContext!.cancelSession(session);
      return;
    }
    if (dragConfiguration != null) {
      final rawConfiguration =
          await dragConfiguration.intoRaw(devicePixelRatio);
      if (buildContext.mounted) {
        session.dragCompleted.addListener(() {
          rawConfiguration.disposeImages();
        });
        await context.startDrag(
            buildContext: buildContext,
            session: session,
            configuration: rawConfiguration,
            position: position);
      } else {
        rawConfiguration.disposeImages();
      }
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
    required this.hitTestBehavior,
    required super.child,
  });

  final HitTestBehavior hitTestBehavior;
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
          recognizer.onStart = (offset) => maybeStartDrag(
                context,
                recognizer.lastPointer,
                offset,
                devicePixelRatio,
              );
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
    required this.hitTestBehavior,
    required super.dragConfiguration,
    required this.isLocationDraggable,
    required super.child,
  });

  final HitTestBehavior hitTestBehavior;
  final LocationIsDraggable isLocationDraggable;

  @override
  Widget build(BuildContext context) {
    return MultiTouchDetector(
      child: RawGestureDetector(
        behavior: hitTestBehavior,
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
      ),
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
