import 'dart:math' as math;

import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:pixel_snap/pixel_snap.dart';

class AnimatedMenuLayoutData extends ImplicitlyAnimatedWidget {
  const AnimatedMenuLayoutData({
    super.key,
    required this.destinationOffset,
    required this.sourceRect,
    required this.transition,
    required this.child,
    super.curve,
    required super.duration,
    this.onTransitionedToZero,
  });

  final VoidCallback? onTransitionedToZero;

  /// Source rect for transition
  final Rect sourceRect;

  /// Destination offset applied to source rect
  final Offset destinationOffset;

  /// Transition from source rect to actual position (0 - 1)
  final double transition;

  final Widget child;

  @override
  AnimatedWidgetBaseState<AnimatedMenuLayoutData> createState() =>
      _AnimatedMenuLayoutDataState();
}

class _AnimatedMenuLayoutDataState
    extends AnimatedWidgetBaseState<AnimatedMenuLayoutData> {
  Tween<double>? _transition;

  @override
  void initState() {
    super.initState();
    controller.addStatusListener((status) {
      if (status == AnimationStatus.completed) {
        if (_transition?.evaluate(animation) == 0) {
          widget.onTransitionedToZero?.call();
        }
      }
    });
  }

  @override
  void forEachTween(TweenVisitor<dynamic> visitor) {
    _transition = visitor(_transition, widget.transition,
            (dynamic value) => Tween<double>(begin: value as double))
        as Tween<double>?;
  }

  @override
  Widget build(BuildContext context) {
    return MenuLayoutData(
      destinationOffset: widget.destinationOffset,
      sourceRect: widget.sourceRect,
      transition: _transition?.evaluate(animation) ?? 0,
      child: widget.child,
    );
  }
}

class MenuLayoutData extends ParentDataWidget<_MenuLayoutParentData> {
  const MenuLayoutData({
    super.key,
    required this.destinationOffset,
    required this.sourceRect,
    required this.transition,
    required super.child,
  });

  /// Source rect for transition
  final Rect sourceRect;

  /// Destination offset applied to source rect
  final Offset destinationOffset;

  /// Transition from source rect to actual position (0 - 1)
  final double transition;

  @override
  void applyParentData(RenderObject renderObject) {
    final _MenuLayoutParentData parentData =
        renderObject.parentData! as _MenuLayoutParentData;

    bool needsLayout = false;

    if (parentData.sourceRect != sourceRect) {
      needsLayout = true;
      parentData.sourceRect = sourceRect;
    }

    if (parentData.destinationOffset != destinationOffset) {
      needsLayout = true;
      parentData.destinationOffset = destinationOffset;
    }

    if (parentData.transition != transition) {
      needsLayout = true;
      parentData.transition = transition;
    }

    if (needsLayout) {
      final AbstractNode? targetParent = renderObject.parent;
      if (targetParent is RenderObject) {
        targetParent.markNeedsLayout();
      }
    }
  }

  @override
  Type get debugTypicalAncestorWidgetClass => MenuLayout;

  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(DiagnosticsProperty<Rect>('sourceRect', sourceRect));
    properties.add(
        DiagnosticsProperty<Offset>('destinationOffset', destinationOffset));
    properties.add(DiagnosticsProperty<double>('transition', transition));
  }
}

class _MenuLayoutParentData extends ContainerBoxParentData<RenderBox> {
  Rect? sourceRect;
  Offset? destinationOffset;
  double? transition;
}

class MenuLayout extends MultiChildRenderObjectWidget {
  // TODO(knopp): Remove when migrated to 3.10
  // ignore: prefer_const_constructors_in_immutables
  MenuLayout({
    super.key,
    super.children,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderMenuLayout(
      pixelSnap: PixelSnap.of(context),
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    super.updateRenderObject(context, renderObject);
    (renderObject as _RenderMenuLayout).pixelSnap = PixelSnap.of(context);
  }
}

class _RenderMenuLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _MenuLayoutParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _MenuLayoutParentData> {
  _RenderMenuLayout({
    required PixelSnap pixelSnap,
  }) : _pixelSnap = pixelSnap;

  PixelSnap _pixelSnap;

  set pixelSnap(PixelSnap value) {
    if (_pixelSnap != value) {
      _pixelSnap = value;
      markNeedsLayout();
    }
  }

  PixelSnap get pixelSnap => _pixelSnap;

  @override
  void setupParentData(covariant RenderObject child) {
    if (child.parentData is! _MenuLayoutParentData) {
      child.parentData = _MenuLayoutParentData();
    }
  }

  @override
  void performLayout() {
    RenderBox? child = firstChild;
    var size = Size.zero;
    int index = 0;
    while (child != null) {
      final _MenuLayoutParentData parentData =
          child.parentData! as _MenuLayoutParentData;

      final depth = math.min(index, 3);
      final minTop = 10.0 * depth;

      final BoxConstraints childConstraints =
          constraints.loosen().deflate(EdgeInsets.only(top: minTop));
      child.layout(childConstraints, parentUsesSize: true);

      final sourceRect = parentData.sourceRect ?? Rect.zero;
      var destinationRect = Rect.fromLTWH(
        parentData.destinationOffset?.dx ?? 0.0,
        sourceRect.top + (parentData.destinationOffset?.dy ?? 0.0),
        child.size.width,
        child.size.height,
      );

      destinationRect = destinationRect.translate(
        math.min(constraints.maxWidth - destinationRect.right, 0),
        math.min(constraints.maxHeight - destinationRect.bottom, 0),
      );

      destinationRect = destinationRect.translate(
        0,
        math.max(minTop - destinationRect.top, 0),
      );

      final Rect actualRect;
      final transition = parentData.transition ?? 0;
      if (transition < 1) {
        final minHeight = child.getMinIntrinsicHeight(child.size.width);
        final adjustedSourceRect = Rect.fromCenter(
          center: sourceRect.center,
          width: destinationRect.width,
          height: minHeight,
        );
        actualRect = Rect.lerp(
            adjustedSourceRect, destinationRect, parentData.transition ?? 0.0)!;
        child.layout(BoxConstraints.tight(actualRect.size));
      } else {
        actualRect = destinationRect;
      }

      size = Size(
        math.max(size.width, actualRect.right),
        math.max(size.height, actualRect.bottom),
      );
      parentData.offset = actualRect.topLeft.pixelSnap(pixelSnap);
      child = childAfter(child);
      ++index;
    }

    this.size = constraints.constrain(size);
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    defaultPaint(context, offset);
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    RenderBox? child = lastChild;
    while (child != null) {
      // The x, y parameters have the top left of the node's box as the origin.
      final _MenuLayoutParentData childParentData =
          child.parentData! as _MenuLayoutParentData;
      if ((childParentData.transition ?? 0.0) > 0.5) {
        final bool isHit = result.addWithPaintOffset(
          offset: childParentData.offset,
          position: position,
          hitTest: (BoxHitTestResult result, Offset transformed) {
            assert(transformed == position - childParentData.offset);
            return child!.hitTest(result, position: transformed);
          },
        );
        if (isHit) {
          return true;
        }
      }
      child = childParentData.previousSibling;
    }
    return false;
  }
}
