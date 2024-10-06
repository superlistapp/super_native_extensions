import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../repaint_boundary.dart';
import 'widget_snapshot.dart';
import 'widget_snapshotter.dart';

class WidgetSnapshotterStateImpl extends WidgetSnapshotterState {
  @override
  void registerWidget(Object key, Widget? widget) {
    // We can't use our "real" render object for snapshot with html renderer
    // so we force another instance of widget rendered.
    if (widget == null && !WidgetSnapshotter.snapshotToImageSupported()) {
      widget = this.widget.child;
    }

    final registeredWidget = _registeredWidgets[key];
    if (registeredWidget != null) {
      ++registeredWidget.retainCount;
    } else {
      setState(() {
        _registeredWidgets[key] = _RegisteredWidget(widget);
      });
    }
  }

  @override
  Future<TargetedWidgetSnapshot?> getSnapshot(
    Offset location,
    Object key,
    ValueGetter<Widget?> widgetBuilder,
  ) async {
    registerWidget(key, widgetBuilder.call());
    final res = await _getSnapshotInner(location, key);

    if (res != null && res.snapshot.isImage) {
      unregisterWidget(key);
    } else if (res != null) {
      bool unregisted = false;
      void unregister() {
        WidgetsBinding.instance.addPostFrameCallback((_) {
          if (!unregisted) {
            unregisted = true;
            unregisterWidget(key);
          }
        });
      }

      res.snapshot.onDisposed.addListener(unregister);
      res.snapshot.onRenderObjectRequested.addListener(unregister);
    }
    return res;
  }

  @override
  void unregisterWidget(Object key) {
    final registeredWidget = _registeredWidgets[key];
    if (registeredWidget != null) {
      if (--registeredWidget.retainCount == 0) {
        setState(() {
          _registeredWidgets.remove(key);
        });
      }
    }
  }

  Future<TargetedWidgetSnapshot?> _getSnapshotInner(
    Offset location,
    Object key,
  ) {
    final completer = Completer<TargetedWidgetSnapshot?>();
    _pendingSnapshots.add(
        _PendingSnapshot(key: key, location: location, completer: completer));
    _checkSnapshots();
    return completer.future;
  }

  void _checkSnapshots() {
    // Remove pending snapshots for widgets that are no longer registered.
    _pendingSnapshots.removeWhere(
      (element) {
        if (_registeredWidgets[element.key] == null) {
          element.complete(Future.value(null));
          return true;
        } else {
          return false;
        }
      },
    );
    if (_pendingSnapshots.isEmpty) {
      return;
    }
    if (!mounted) {
      for (final snapshot in _pendingSnapshots) {
        snapshot.complete(Future.value(null));
      }
      _pendingSnapshots.clear();
      return;
    }
    // We have pending snapshot for widget that we haven't built yet
    if (_pendingSnapshots.any((s) => _getRenderObject(s.key) == null)) {
      setState(() {});
      // TODO(knopp): This is fragile and the underlying reason for the deadlock
      // needs to be investigates.
      // On iOS 18 next frame vsync never comes while the run loop is being
      // polled. This is a ugly workaround to make sure that we do not
      // deadlock.
      WidgetsBinding.instance.scheduleWarmUpFrame();
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshots();
      });
      return;
    }
    for (final s in _pendingSnapshots) {
      Translation? translation;
      final renderObject = _getRenderObject(s.key);
      if (_registeredWidgets[s.key]?.widget != null) {
        final snapshotLayoutRenderBox = _registeredWidgets[s.key]
            ?.repaintBoundaryKey
            .currentContext
            ?.findAncestorRenderObjectOfType<_SnapshotLayoutRenderBox>();
        final parentData = snapshotLayoutRenderBox?.parentData;
        if (parentData is _ParentData) {
          translation = parentData.translation;
        }
      }
      if (renderObject != null && renderObject.canGetSnapshot) {
        final snapshot = _getSnapshot(
          context,
          renderObject,
          s.location,
          translation,
        ).then((value) {
          value.snapshot.debugKey = s.key;
          return value;
        });
        s.complete(snapshot);
      } else {
        s.complete(Future.value(null));
      }
    }
    _pendingSnapshots.clear();
    setState(() {});
  }

  RenderBetterRepaintBoundary? _getRenderObject(Object key) {
    final registeredWidget = _registeredWidgets[key];
    if (registeredWidget == null) {
      return null;
    }
    final object = registeredWidget.widget == null
        ? _childSnapshotKey.currentContext?.findRenderObject()
        : registeredWidget.repaintBoundaryKey.currentContext
            ?.findRenderObject();
    return object is RenderBetterRepaintBoundary ? object : null;
  }

  final _childKey = GlobalKey();
  final _childSnapshotKey = GlobalKey();
  final _registeredWidgets = <Object, _RegisteredWidget>{};
  final _pendingSnapshots = <_PendingSnapshot>[];

  @override
  Widget build(BuildContext context) {
    if (_registeredWidgets.isEmpty) {
      return KeyedSubtree(
        key: _childKey,
        child: widget.child,
      );
    } else {
      final needRepaintBoundaryForDefaultChild =
          _registeredWidgets.values.any((a) => a.widget == null);
      return _SnapshotLayout(children: [
        if (needRepaintBoundaryForDefaultChild)
          BetterRepaintBoundary(
            key: _childSnapshotKey,
            child: KeyedSubtree(
              key: _childKey,
              child: widget.child,
            ),
          ),
        if (!needRepaintBoundaryForDefaultChild)
          KeyedSubtree(
            key: _childKey,
            child: widget.child,
          ),
        for (final w in _registeredWidgets.entries)
          _SnapshotLayoutRenderObjectWidget(
            key: w.value.renderObjectKey,
            debugSnapshotKey: w.key,
            child: ClipRect(
              clipper: const _ZeroClipper(),
              child: BetterRepaintBoundary(
                key: w.value.repaintBoundaryKey,
                child: w.value.widget,
              ),
            ),
          )
      ]);
    }
  }
}

class _RegisteredWidget {
  _RegisteredWidget(this.widget);

  int retainCount = 1;

  // If null, default widget will be used.
  final Widget? widget;
  final renderObjectKey = GlobalKey();
  final repaintBoundaryKey = GlobalKey();
}

class _PendingSnapshot {
  _PendingSnapshot({
    required this.key,
    required this.location,
    required this.completer,
  });

  void complete(Future<TargetedWidgetSnapshot?> image) async {
    // Weirdly simply calling completer.complete(image) will resolve the future
    // synchronously, which is not expected and may result in another
    // getSnapshot() call in the meanwhile thus concurrent modification exception.
    try {
      final value = await image;
      completer.complete(value);
    } catch (e, st) {
      completer.completeError(e, st);
    }
  }

  final Object key;
  final Offset location;
  final Completer<TargetedWidgetSnapshot?> completer;
}

Future<TargetedWidgetSnapshot> _getSnapshot(
    BuildContext context,
    RenderBetterRepaintBoundary renderObject,
    Offset location,
    Offset Function(Rect rect, Offset offset)? translation) async {
  final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
  ui.Image? image;
  if (WidgetSnapshotter.snapshotToImageSupported()) {
    image = renderObject.toImageSync(pixelRatio: devicePixelRatio);
    image.devicePixelRatio = devicePixelRatio;
  }
  final transform = renderObject.getTransformTo(null);
  final r =
      Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height);

  var offset = Offset.zero;
  if (translation != null) {
    final inverted = transform.clone()..invert();
    final dragLocation = MatrixUtils.transformPoint(inverted, location);
    offset = translation(r, dragLocation);
  }

  final rect = MatrixUtils.transformRect(transform, r.shift(offset));
  if (image != null) {
    return TargetedWidgetSnapshot(
      WidgetSnapshot.image(image),
      rect,
    );
  } else {
    final size = Size(renderObject.size.width, renderObject.size.height);
    return TargetedWidgetSnapshot(
      WidgetSnapshot.renderObject(
        renderObject,
        Offset.zero & size,
      ),
      rect,
    );
  }
}

class _ZeroClipper extends CustomClipper<Rect> {
  const _ZeroClipper();

  @override
  Rect getClip(Size size) {
    return Rect.zero;
  }

  @override
  bool shouldReclip(covariant CustomClipper<Rect> oldClipper) {
    return false;
  }
}

class _SnapshotLayoutRenderObjectWidget extends SingleChildRenderObjectWidget {
  const _SnapshotLayoutRenderObjectWidget({
    required super.child,
    required super.key,
    required this.debugSnapshotKey,
  });

  final Object debugSnapshotKey;

  @override
  RenderObject createRenderObject(BuildContext context) {
    final res = _SnapshotLayoutRenderBox();
    res.debugSnapshotKey = debugSnapshotKey;
    return res;
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant _SnapshotLayoutRenderBox renderObject) {
    renderObject.debugSnapshotKey = debugSnapshotKey;
  }
}

class _SnapshotLayoutRenderBox extends RenderProxyBox {
  Object? debugSnapshotKey;
}

class SnapshotSettingsState extends State<SnapshotSettings> {
  @override
  void initState() {
    super.initState();
    final settings =
        context.findAncestorRenderObjectOfType<_SnapshotLayoutRenderBox>();
    if (settings != null) {
      final parentData = settings.parentData;
      if (parentData is _ParentData) {
        parentData.constraintsTransform = widget.constraintsTransform;
        parentData.translation = widget.translation;
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    return widget.child;
  }
}

class _SnapshotLayout extends MultiChildRenderObjectWidget {
  const _SnapshotLayout({
    // ignore: unused_element
    super.key,
    required super.children,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderSnapshotLayout();
  }
}

class _ParentData extends ContainerBoxParentData<RenderBox> {
  Key? key;
  BoxConstraintsTransform? constraintsTransform;
  Translation? translation;
}

class _RenderSnapshotLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox,
            ContainerBoxParentData<RenderBox>>,
        RenderBoxContainerDefaultsMixin<RenderBox,
            ContainerBoxParentData<RenderBox>> {
  @override
  void setupParentData(RenderBox child) {
    if (child.parentData is! _ParentData) {
      child.parentData = _ParentData();
    }
  }

  @override
  double computeMaxIntrinsicWidth(double height) {
    return firstChild?.computeMaxIntrinsicWidth(height) ?? 0.0;
  }

  @override
  double computeMinIntrinsicWidth(double height) {
    return firstChild?.computeMinIntrinsicWidth(height) ?? 0.0;
  }

  @override
  double computeMaxIntrinsicHeight(double width) {
    return firstChild?.computeMaxIntrinsicHeight(width) ?? 0.0;
  }

  @override
  double computeMinIntrinsicHeight(double width) {
    return firstChild?.computeMinIntrinsicHeight(width) ?? 0.0;
  }

  @override
  Size computeDryLayout(BoxConstraints constraints) {
    return firstChild?.computeDryLayout(constraints) ?? Size.zero;
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    return firstChild?.hitTest(result, position: position) ?? false;
  }

  @override
  void performLayout() {
    RenderBox? child = firstChild;
    if (child != null) {
      child.layout(constraints, parentUsesSize: true);
      size = child.size;

      while (true) {
        child = (child!.parentData as _ParentData).nextSibling;

        if (child == null) {
          break;
        } else {
          final parentData = child.parentData as _ParentData;
          final constraints =
              parentData.constraintsTransform?.call(this.constraints) ??
                  this.constraints;
          child.layout(constraints, parentUsesSize: false);
        }
      }
    } else {
      size = Size.zero;
    }
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    defaultPaint(context, offset);
  }
}
