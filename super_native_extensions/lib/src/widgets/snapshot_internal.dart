import 'dart:async';

import 'package:collection/collection.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../api_model.dart';
import 'snapshot.dart';

class RawCustomSnapshotWidgetState extends State<RawCustomSnapshotWidget>
    implements Snapshotter {
  @override
  Widget build(BuildContext context) {
    return widget.child;
  }

  @override
  Future<TargetedImage?> getSnapshot(Offset location, SnapshotType? type) {
    return widget.onGetSnapshot(location, type);
  }

  @override
  void prepare(Set<SnapshotType> types) {
    return widget.onPrepare(types);
  }

  @override
  void unprepare() {
    widget.onUnprepare();
  }
}

class _PendingSnapshot {
  _PendingSnapshot(this.type, this.location);

  final SnapshotType? type;
  final Offset location;
  final completers = <Completer<TargetedImage?>>[];

  void complete(Future<TargetedImage?> image) async {
    // Weirdly simply calling completer.complete(image) will resolve the future
    // synchronously, which is not expected and may result in another
    // getSnapshot() call in the meanwhile thus concurrent modification exception.
    try {
      final value = await image;
      for (final completer in completers) {
        completer.complete(value);
      }
    } catch (e, st) {
      for (final completer in completers) {
        completer.completeError(e, st);
      }
    }
  }
}

Future<TargetedImage> _getSnapshot(
    BuildContext context,
    RenderRepaintBoundary renderObject,
    Offset location,
    Offset Function(Rect rect, Offset offset)? translation) async {
  final image = renderObject.toImageSync(
      pixelRatio: MediaQuery.of(context).devicePixelRatio);
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
  return TargetedImage(image, rect);
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
  const _SnapshotLayoutRenderObjectWidget({required super.child});

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _SnapshotLayoutRenderBox();
  }
}

class _SnapshotLayoutRenderBox extends RenderProxyBox {}

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

class CustomSnapshotWidgetState extends State<CustomSnapshotWidget> {
  final _lastBuiltTypes = <SnapshotType>{};

  @override
  Widget build(BuildContext context) {
    return RawCustomSnapshotWidget(
      onGetSnapshot: getSnapshot,
      onPrepare: prepare,
      onUnprepare: unprepare,
      child: Builder(builder: (context) {
        _lastBuiltTypes.clear();
        if (_prepared.isEmpty && _pendingSnapshots.isEmpty) {
          return KeyedSubtree(
            key: _contentKey,
            child: widget.child,
          );
        } else {
          for (final p in _prepared) {
            _lastBuiltTypes.add(p);
          }
          for (final p in _pendingSnapshots) {
            if (p.type != null) {
              _lastBuiltTypes.add(p.type!);
            }
          }
          Widget? typeToWidget(SnapshotType type) {
            final w = widget.snapshotBuilder(context, widget.child, type);
            if (w == null) {
              return null;
            }
            return _SnapshotLayoutRenderObjectWidget(
              child: ClipRect(
                clipper: const _ZeroClipper(),
                child: RepaintBoundary(
                  key: _keys[type],
                  child: w,
                ),
              ),
            );
          }

          return _SnapshotLayout(
            children: [
              RepaintBoundary(
                key: _defaultKey,
                child: KeyedSubtree(
                  key: _contentKey,
                  child: widget.child,
                ),
              ),
              ..._lastBuiltTypes.map(typeToWidget).whereNotNull(),
            ],
          );
        }
      }),
    );
  }

  final _prepared = <SnapshotType>{};

  final _pendingSnapshots = <_PendingSnapshot>[];

  final _contentKey = GlobalKey();

  final _defaultKey = GlobalKey();

  final _keys = {
    for (final type in SnapshotType.values) type: GlobalKey(),
  };

  RenderRepaintBoundary? _getRenderObject(SnapshotType? type) {
    final object = type != null
        ? _keys[type]?.currentContext?.findRenderObject()
        : _defaultKey.currentContext?.findRenderObject();
    return object is RenderRepaintBoundary ? object : null;
  }

  void prepare(Set<SnapshotType> types) {
    if (_prepared.containsAll(types)) {
      return;
    }
    setState(() {
      _prepared.addAll(types);
    });
  }

  void unprepare() {
    if (_prepared.isEmpty) {
      return;
    }
    setState(() {
      _prepared.clear();
    });
  }

  void _checkSnapshots() {
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
    // If we have pending snapshot of type for which we didn't try building
    // a widget yet, we need to wait for the next frame.
    if (_getRenderObject(null) == null ||
        _pendingSnapshots.any(
          (s) =>
              s.type != null && //
              !_lastBuiltTypes.contains(s.type),
        )) {
      setState(() {});
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshots();
      });
      return;
    }
    for (final s in _pendingSnapshots) {
      Translation? translation;
      final renderObject = _getRenderObject(s.type);
      if (s.type != null) {
        final snapshotLayoutRenderBox = _keys[s.type]
            ?.currentContext
            ?.findAncestorRenderObjectOfType<_SnapshotLayoutRenderBox>();
        final parentData = snapshotLayoutRenderBox?.parentData;
        if (parentData is _ParentData) {
          translation = parentData.translation;
        }
      }
      if (renderObject != null) {
        s.complete(_getSnapshot(
          context,
          renderObject,
          s.location,
          translation,
        ));
      } else {
        s.complete(Future.value(null));
      }
    }
    _pendingSnapshots.clear();
    setState(() {});
  }

  Future<TargetedImage?> getSnapshot(Offset location, SnapshotType? type) {
    final completer = Completer<TargetedImage?>();
    var snapshot = _pendingSnapshots.firstWhereOrNull((s) => s.type == type);
    if (snapshot == null) {
      snapshot = _PendingSnapshot(type, location);
      _pendingSnapshots.add(snapshot);
    }
    snapshot.completers.add(completer);
    // Let other sites request snapshot before checking for completion.
    Future.microtask(_checkSnapshots);
    return completer.future;
  }
}

class FallbackSnapshotWidgetState extends State<FallbackSnapshotWidget>
    implements Snapshotter {
  final _contentKey = GlobalKey();
  final _repaintBoundaryKey = GlobalKey();

  final _pendingSnapshots = <_PendingSnapshot>[];

  bool _prepared = false;

  @override
  Widget build(BuildContext context) {
    if (!_prepared && _pendingSnapshots.isEmpty) {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
    if (_prepared || _pendingSnapshots.isNotEmpty) {
      return RepaintBoundary(
        key: _repaintBoundaryKey,
        child: KeyedSubtree(key: _contentKey, child: widget.child),
      );
    } else {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
  }

  @override
  Future<TargetedImage?> getSnapshot(Offset location, SnapshotType? type) {
    if (type != null) {
      return Future.value(null);
    }

    final completer = Completer<TargetedImage>();
    var snapshot = _pendingSnapshots.firstWhereOrNull((s) => s.type == type);
    if (snapshot == null) {
      snapshot = _PendingSnapshot(type, location);
      _pendingSnapshots.add(snapshot);
    }
    snapshot.completers.add(completer);
    _checkSnapshot();
    return completer.future;
  }

  void _checkSnapshot() {
    if (!mounted) {
      for (final snapshot in _pendingSnapshots) {
        snapshot.complete(Future.value(null));
      }
      _pendingSnapshots.clear();
      return;
    }
    final object = _repaintBoundaryKey.currentContext?.findRenderObject();
    if (object is RenderRepaintBoundary) {
      for (final snapshot in _pendingSnapshots) {
        final image = _getSnapshot(context, object, snapshot.location, null);
        snapshot.complete(image);
      }
      _pendingSnapshots.clear();
      setState(() {});
    } else {
      setState(() {});
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshot();
      });
    }
  }

  @override
  void prepare(Set<SnapshotType> types) {
    if (_prepared) {
      return;
    }
    setState(() {
      _prepared = true;
    });
  }

  @override
  void unprepare() {
    if (!_prepared) {
      return;
    }
    setState(() {
      _prepared = false;
    });
  }
}

class _SnapshotLayout extends MultiChildRenderObjectWidget {
  _SnapshotLayout({
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
          child.layout(constraints);
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
