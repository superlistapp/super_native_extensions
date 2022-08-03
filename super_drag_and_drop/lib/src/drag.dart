import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
export 'package:super_native_extensions/raw_drag_drop.dart' show DropOperation;

import 'drag_configuration.dart';
import 'into_raw.dart';

class _DragContextDelegate implements raw.DragContextDelegate {
  @override
  Future<raw.DragConfiguration?> getConfigurationForDragRequest({
    required Offset location,
    required raw.DragSession session,
  }) async {
    final hitTest = HitTestResult();
    print('Testing at $location');
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

bool _initialized = false;
raw.DragContext? _dragContext;

Future<void> _initializeIfNeeded() async {
  if (!_initialized) {
    _initialized = true;
    _dragContext = await raw.DragContext.instance();
    _dragContext!.delegate = _DragContextDelegate();
  }
}

class DragSession {
  DragSession._(raw.DragSession session) : _session = session;

  Listenable get dragStarted => _session.dragStarted;
  ValueListenable<raw.DropOperation?> get dragCompleted =>
      _session.dragCompleted;
  ValueListenable<Offset?> get lastScreenLocation =>
      _session.lastScreenLocation;

  final raw.DragSession _session;
}

typedef IsLocationDraggableCallback = bool Function(Offset position);
typedef GetDragConfigurationCallback = Future<DragConfiguration?> Function(
    Offset position, DragSession session);

class BaseDraggable extends StatelessWidget {
  const BaseDraggable({
    super.key,
    required this.child,
    required this.onGetDragConfiguration,
    this.onIsLocationDraggable = _defaultIsLocationDraggable,
  });

  static bool _defaultIsLocationDraggable(Offset offset) => true;

  final Widget child;
  final GetDragConfigurationCallback onGetDragConfiguration;
  final IsLocationDraggableCallback onIsLocationDraggable;

  @override
  Widget build(BuildContext context) {
    var child = this.child;
    if (defaultTargetPlatform == TargetPlatform.iOS) {
      // handled by delegate
    } else {
      child = _DesktopDragDetector(
          onGetDragConfiguration: onGetDragConfiguration, child: child);
    }
    return _BaseDragableRenderObject(
      onGetDragConfiguration: onGetDragConfiguration,
      onIsLocationDraggable: onIsLocationDraggable,
      child: child,
    );
  }
}

typedef DragItemCallback = Future<DragItem?> Function(
    AsyncValueGetter<DragImage> snapshot);

class DragItemWidget extends StatefulWidget {
  const DragItemWidget({
    super.key,
    required this.child,
    required this.onGetItem,
    required this.onGetAllowedOperations,
  });

  final Widget child;
  final DragItemCallback onGetItem;
  final ValueGetter<List<raw.DropOperation>> onGetAllowedOperations;

  @override
  State<StatefulWidget> createState() => DragItemWidgetState();
}

class DragItemWidgetState extends State<DragItemWidget> {
  final repaintBoundary = GlobalKey();

  Future<DragImage> _getSnapshot() async {
    final renderObject = repaintBoundary.currentContext?.findRenderObject()
        as RenderRepaintBoundary;
    final image = await renderObject.toImage(
        pixelRatio: MediaQuery.of(context).devicePixelRatio);
    final transform = renderObject.getTransformTo(null);
    final r =
        Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height);
    final rect = MatrixUtils.transformRect(transform, r);
    return DragImage(image, rect);
  }

  Future<DragItem?> createItem() async {
    return widget.onGetItem(_getSnapshot);
  }

  Future<List<raw.DropOperation>> getAllowedOperations() async {
    return widget.onGetAllowedOperations();
  }

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      key: repaintBoundary,
      child: widget.child,
    );
  }
}

class _DesktopDragDetector extends StatelessWidget {
  final Widget child;
  final GetDragConfigurationCallback onGetDragConfiguration;

  const _DesktopDragDetector({
    required this.onGetDragConfiguration,
    required this.child,
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
              (offset) => _maybeStartDrag(offset, devicePixelRatio);
        })
      },
      child: child,
    );
  }

  Drag? _maybeStartDrag(Offset offset, double devicePixelRatio) {
    final dragContext = _dragContext;
    if (dragContext != null) {
      final session = dragContext.newSession();
      _maybeStartDragWithSession(
          dragContext, offset, session, devicePixelRatio);
      return session is Drag ? session as Drag : null;
    } else {
      return null;
    }
  }

  void _maybeStartDragWithSession(raw.DragContext context, Offset offset,
      raw.DragSession session, double devicePixelRatio) async {
    final dragConfiguration =
        await onGetDragConfiguration(offset, DragSession._(session));
    if (dragConfiguration != null) {
      context.startDrag(
          session: session,
          configuration: await dragConfiguration.intoRaw(devicePixelRatio),
          position: offset);
    }
  }
}

class _BaseDragableRenderObject extends SingleChildRenderObjectWidget {
  const _BaseDragableRenderObject({
    required super.child,
    required this.onGetDragConfiguration,
    required this.onIsLocationDraggable,
  });

  final GetDragConfigurationCallback onGetDragConfiguration;
  final IsLocationDraggableCallback onIsLocationDraggable;

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

  @override
  bool hitTest(BoxHitTestResult result, {required ui.Offset position}) {
    final res = super.hitTest(
      result,
      position: position,
    );
    print('RES $res');
    return res;
  }

  double devicePixelRatio;
  GetDragConfigurationCallback onGetDragConfiguration;
  IsLocationDraggableCallback onIsLocationDraggable;
}

typedef GetDragItems = List<DragItemWidgetState> Function(BuildContext context);

class SimpleDraggable extends StatelessWidget {
  const SimpleDraggable({
    super.key,
    required this.child,
    this.onGetDragItems = _defaultGetDragItems,
  });

  final Widget child;
  final GetDragItems onGetDragItems;

  static List<DragItemWidgetState> _defaultGetDragItems(BuildContext context) {
    final state = context.findAncestorStateOfType<DragItemWidgetState>();
    if (state != null) {
      return [state];
    } else {
      throw Exception('SimpleDraggable must be placed inside a DragItemWidget');
    }
  }

  Future<DragConfiguration?> dragConfigurationForItems(
      List<DragItemWidgetState> items) async {
    List<raw.DropOperation>? allowedOperations;
    for (final item in items) {
      if (allowedOperations == null) {
        allowedOperations = List.from(await item.getAllowedOperations());
      } else {
        final itemOperations = await item.getAllowedOperations();
        allowedOperations
            .retainWhere((element) => itemOperations.contains(element));
      }
    }

    if (allowedOperations?.isNotEmpty == true) {
      final dragItems = <DragItem>[];
      for (final item in items) {
        final dragItem = await item.createItem();
        if (dragItem != null) {
          dragItems.add(dragItem);
        }
      }
      if (dragItems.isNotEmpty) {
        return DragConfiguration(
            items: dragItems, allowedOperations: allowedOperations!);
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    return BaseDraggable(
        child: child,
        onGetDragConfiguration: (_, __) async {
          final items = onGetDragItems(context);
          return dragConfigurationForItems(items);
        });
  }
}
