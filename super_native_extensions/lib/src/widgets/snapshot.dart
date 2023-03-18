import 'dart:async';

import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

import '../api_model.dart';

enum SnapshotType {
  /// Snapshot used for lift animation on iOS.
  lift,

  /// Snapshot used during dragging.
  drag,
}

typedef TranslationProvider = Offset Function(
  /// Snapshot rectangle in local coordinates.
  Rect rect,

  /// Drag position within the rectangle.
  Offset dragPosition,

  /// Type of snapshot.
  SnapshotType type,
);

typedef ConstraintsTransformProvider = BoxConstraints Function(
  BoxConstraints constraints,
  SnapshotType type,
);

typedef SnapshotBuilder = Widget Function(
  BuildContext context,

  /// Type of snapshot currently being built or `null` when building
  /// normal child widget.
  SnapshotType? type,
);

/// Widget that provides custom dragging snapshots.
class CustomSnapshotWidget extends StatefulWidget {
  const CustomSnapshotWidget({
    super.key,
    this.supportedTypes = const {SnapshotType.drag},
    required this.builder,
    this.translation,
    this.constraintsTransform,
  });

  /// Set of supported snapshot types. The builder will be called
  /// only for these types.
  final Set<SnapshotType> supportedTypes;

  /// Builder that creates the widget that will be used as a snapshot.
  /// The builder will be called with `null` type when building normal
  /// child widget.
  final SnapshotBuilder builder;

  /// Allows to transform snapshot location.
  final TranslationProvider? translation;

  /// Allows to transform constraints for snapshot widget. The resulting
  /// constraints may exceed parent constraints without causing an error.
  final ConstraintsTransformProvider? constraintsTransform;

  @override
  State<CustomSnapshotWidget> createState() => _CustomSnapshotWidgetState();
}

abstract class Snapshotter {
  static Snapshotter? of(BuildContext context) {
    final real = context.findAncestorStateOfType<_CustomSnapshotWidgetState>();
    if (real != null) {
      return real;
    } else {
      return context.findAncestorStateOfType<_FallbackSnapshotWidgetState>();
    }
  }

  set armed(bool armed);

  Future<TargettedImage?> getSnapshot(Offset location, SnapshotType? type);
}

class _PendingSnapshot {
  _PendingSnapshot(this.type, this.location, this.completer);

  final SnapshotType? type;
  final Offset location;
  final Completer<TargettedImage?> completer;
}

TargettedImage _getSnapshot(
    BuildContext context,
    RenderRepaintBoundary renderObject,
    Offset location,
    Offset Function(Rect rect, Offset offset)? translation) {
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
  return TargettedImage(image, rect);
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

class _CustomSnapshotWidgetState extends State<CustomSnapshotWidget>
    implements Snapshotter {
  BoxConstraintsTransform? _constrainTransformForType(SnapshotType type) {
    if (widget.constraintsTransform == null) {
      return null;
    } else {
      return (constraints) => widget.constraintsTransform!(constraints, type);
    }
  }

  @override
  Widget build(BuildContext context) {
    return Builder(builder: (context) {
      if (!_armed && _pendingSnapshots.isEmpty) {
        return KeyedSubtree(
          key: _contentKey,
          child: widget.builder(context, null),
        );
      } else {
        return _SnapshotLayout(
          children: [
            RepaintBoundary(
              key: _defaultKey,
              child: KeyedSubtree(
                key: _contentKey,
                child: widget.builder(context, null),
              ),
            ),
            for (final type in widget.supportedTypes)
              _SnapshotLayoutParentDataWidget(
                constraintsTransform: _constrainTransformForType(type),
                child: ClipRect(
                  clipper: const _ZeroClipper(),
                  child: RepaintBoundary(
                    key: _keys[type],
                    child: widget.builder(context, type),
                  ),
                ),
              ),
          ],
        );
      }
    });
  }

  bool _armed = false;
  final _pendingSnapshots = <_PendingSnapshot>[];

  final _contentKey = GlobalKey();

  final _defaultKey = GlobalKey();

  final _keys = {
    SnapshotType.lift: GlobalKey(),
    SnapshotType.drag: GlobalKey(),
  };

  RenderRepaintBoundary? _getRenderObject(SnapshotType? type) {
    final object = type != null
        ? _keys[type]?.currentContext?.findRenderObject()
        : _defaultKey.currentContext?.findRenderObject();
    return object is RenderRepaintBoundary ? object : null;
  }

  @override
  set armed(bool value) {
    if (_armed != value) {
      setState(() {
        _armed = value;
      });
    }
  }

  void _checkSnapshots() {
    if (_pendingSnapshots.isEmpty) {
      return;
    }
    if (!mounted) {
      for (final snapshot in _pendingSnapshots) {
        snapshot.completer.complete(null);
      }
      _pendingSnapshots.clear();
      return;
    }
    if (_getRenderObject(null) == null) {
      setState(() {});
      WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
        _checkSnapshots();
      });
      return;
    }

    for (final s in _pendingSnapshots) {
      final translation = s.type != null
          ? (Rect rect, Offset offset) =>
              widget.translation?.call(rect, offset, s.type!) ?? Offset.zero
          : null;
      final renderObject = _getRenderObject(s.type);
      if (renderObject != null) {
        s.completer.complete(_getSnapshot(
          context,
          renderObject,
          s.location,
          translation,
        ));
      } else {
        s.completer.complete(null);
      }
    }
    _pendingSnapshots.clear();
    setState(() {});
  }

  @override
  Future<TargettedImage?> getSnapshot(Offset location, SnapshotType? type) {
    final completer = Completer<TargettedImage?>();
    _pendingSnapshots.add(_PendingSnapshot(type, location, completer));
    _checkSnapshots();
    return completer.future;
  }
}

class FallbackSnapshotWidget extends StatefulWidget {
  const FallbackSnapshotWidget({
    super.key,
    required this.child,
  });

  final Widget child;

  @override
  State<FallbackSnapshotWidget> createState() => _FallbackSnapshotWidgetState();
}

class _FallbackSnapshotWidgetState extends State<FallbackSnapshotWidget>
    implements Snapshotter {
  final _contentKey = GlobalKey();
  final _repaintBoundaryKey = GlobalKey();

  final _pendingSnapshots = <_PendingSnapshot>[];

  bool _armed = false;

  @override
  Widget build(BuildContext context) {
    if (!_armed && _pendingSnapshots.isEmpty) {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
    if (_armed || _pendingSnapshots.isNotEmpty) {
      return RepaintBoundary(
        key: _repaintBoundaryKey,
        child: KeyedSubtree(key: _contentKey, child: widget.child),
      );
    } else {
      return KeyedSubtree(key: _contentKey, child: widget.child);
    }
  }

  @override
  Future<TargettedImage?> getSnapshot(Offset location, SnapshotType? type) {
    if (type != null) {
      return Future.value(null);
    }

    final snapshot =
        _PendingSnapshot(null, location, Completer<TargettedImage>());
    _pendingSnapshots.add(snapshot);
    _checkSnapshot();
    return snapshot.completer.future;
  }

  void _checkSnapshot() {
    if (!mounted) {
      for (final snapshot in _pendingSnapshots) {
        snapshot.completer.complete(null);
      }
      _pendingSnapshots.clear();
      return;
    }
    final object = _repaintBoundaryKey.currentContext?.findRenderObject();
    if (object is RenderRepaintBoundary) {
      for (final snapshot in _pendingSnapshots) {
        final image = _getSnapshot(context, object, snapshot.location, null);
        snapshot.completer.complete(image);
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
  set armed(bool value) {
    if (_armed != value) {
      setState(() {
        _armed = value;
      });
    }
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
}

class _SnapshotLayoutParentDataWidget extends ParentDataWidget<_ParentData> {
  const _SnapshotLayoutParentDataWidget({
    this.constraintsTransform,
    required super.child,
  });

  final BoxConstraintsTransform? constraintsTransform;

  @override
  void applyParentData(RenderObject renderObject) {
    final parentData = renderObject.parentData as _ParentData;
    if (parentData.constraintsTransform != constraintsTransform) {
      parentData.constraintsTransform = constraintsTransform;
      final targetParent = renderObject.parent;
      if (targetParent is RenderObject) {
        targetParent.markNeedsLayout();
      }
    }
  }

  @override
  Type get debugTypicalAncestorWidgetClass => _SnapshotLayout;
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
