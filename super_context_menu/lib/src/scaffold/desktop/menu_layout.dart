import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:pixel_snap/pixel_snap.dart';

enum MenuLayoutEdge { left, right }

class _MenuLayoutParentData extends ContainerBoxParentData<RenderBox> {
  Offset? primaryPosition;
  MenuLayoutEdge? primaryEdge;
  Offset? secondaryPosition;
  MenuLayoutEdge? secondaryEdge;

  Offset? resolvedPosition;
  MenuLayoutEdge? resolvedEdge;
}

class MenuLayoutData extends ParentDataWidget<_MenuLayoutParentData> {
  const MenuLayoutData({
    super.key,
    required this.primaryPosition,
    required this.primaryEdge,
    required this.secondaryEdge,
    required this.secondaryPosition,
    required super.child,
  });

  final Offset primaryPosition;
  final MenuLayoutEdge primaryEdge;
  final Offset? secondaryPosition;
  final MenuLayoutEdge? secondaryEdge;

  @override
  void applyParentData(RenderObject renderObject) {
    final _MenuLayoutParentData parentData =
        renderObject.parentData! as _MenuLayoutParentData;

    if (parentData.primaryPosition != primaryPosition) {
      parentData.primaryPosition = primaryPosition;
      parentData.resolvedPosition = null;
    }

    if (parentData.primaryEdge != primaryEdge) {
      parentData.primaryEdge = primaryEdge;
      parentData.resolvedEdge = null;
    }

    if (parentData.secondaryPosition != secondaryPosition) {
      parentData.secondaryPosition = secondaryPosition;
      parentData.resolvedPosition = null;
    }

    if (parentData.secondaryEdge != secondaryEdge) {
      parentData.secondaryEdge = secondaryEdge;
      parentData.resolvedEdge = null;
    }

    if (parentData.resolvedPosition == null ||
        parentData.resolvedEdge == null) {
      final AbstractNode? targetParent = renderObject.parent;
      if (targetParent is RenderObject) {
        targetParent.markNeedsLayout();
      }
    }
  }

  @override
  Type get debugTypicalAncestorWidgetClass => MenuLayout;
}

class MenuLayout extends MultiChildRenderObjectWidget {
  const MenuLayout({
    super.key,
    super.children,
    required this.padding,
  });

  final EdgeInsets padding;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderMenuLayout(
      pixelSnap: PixelSnap.of(context),
      padding: padding,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    (renderObject as _RenderMenuLayout)
      ..padding = padding
      ..pixelSnap = PixelSnap.of(context);
  }
}

extension on Rect {
  bool containsRect(Rect other) {
    return other.left >= left &&
        other.right <= right &&
        other.top >= top &&
        other.bottom <= bottom;
  }

  Rect positionVerticalyWithinBounds(Rect bounds) {
    if (top < bounds.top) {
      return translate(0, bounds.top - top);
    } else if (bottom > bounds.bottom) {
      return translate(0, bounds.bottom - bottom);
    } else {
      return this;
    }
  }

  Rect positionHorizontalyWithinBounds(Rect bounds) {
    if (left < bounds.left) {
      return translate(bounds.left - left, 0);
    } else if (right > bounds.right) {
      return translate(bounds.right - right, 0);
    } else {
      return this;
    }
  }
}

class _RenderMenuLayout extends RenderBox
    with
        ContainerRenderObjectMixin<RenderBox, _MenuLayoutParentData>,
        RenderBoxContainerDefaultsMixin<RenderBox, _MenuLayoutParentData> {
  _RenderMenuLayout({
    required PixelSnap pixelSnap,
    required EdgeInsets padding,
  })  : _pixelSnap = pixelSnap,
        _padding = padding;

  PixelSnap _pixelSnap;

  set pixelSnap(PixelSnap value) {
    if (_pixelSnap != value) {
      _pixelSnap = value;
      markNeedsLayout();
    }
  }

  PixelSnap get pixelSnap => _pixelSnap;

  EdgeInsets _padding;

  set padding(EdgeInsets value) {
    if (_padding != value) {
      _padding = value;
      markNeedsLayout();
    }
  }

  EdgeInsets get padding => _padding;

  @override
  void setupParentData(covariant RenderObject child) {
    if (child.parentData is! _MenuLayoutParentData) {
      child.parentData = _MenuLayoutParentData();
    }
  }

  static Rect _getRect(Size size, Offset position, MenuLayoutEdge edge) {
    switch (edge) {
      case MenuLayoutEdge.left:
        return Rect.fromLTWH(
          position.dx,
          position.dy,
          size.width,
          size.height,
        );
      case MenuLayoutEdge.right:
        return Rect.fromLTWH(
          position.dx - size.width,
          position.dy,
          size.width,
          size.height,
        );
    }
  }

  void _resolvePositionIfNeeded(
      _MenuLayoutParentData parentData, Size childSize, Rect bounds) {
    if (parentData.resolvedPosition == null ||
        parentData.resolvedEdge == null) {
      final primaryRect = _getRect(
        childSize,
        parentData.primaryPosition!,
        parentData.primaryEdge!,
      ).positionVerticalyWithinBounds(bounds);
      if (bounds.containsRect(primaryRect) ||
          parentData.secondaryPosition == null) {
        parentData.resolvedPosition = parentData.primaryPosition;
        parentData.resolvedEdge = parentData.primaryEdge;
      } else {
        final secondaryRect = _getRect(
          childSize,
          parentData.secondaryPosition!,
          parentData.secondaryEdge!,
        ).positionVerticalyWithinBounds(bounds);
        if (bounds.containsRect(secondaryRect)) {
          parentData.resolvedPosition = parentData.secondaryPosition;
          parentData.resolvedEdge = parentData.secondaryEdge;
        } else {
          final primaryCorrected =
              primaryRect.positionHorizontalyWithinBounds(bounds);
          final secondaryCorrected =
              secondaryRect.positionHorizontalyWithinBounds(bounds);

          // Use whichever requires the least amount of movement
          if ((primaryCorrected.center - primaryRect.center).distanceSquared <=
              (secondaryCorrected.center - secondaryRect.center)
                  .distanceSquared) {
            parentData.resolvedPosition = parentData.primaryPosition;
            parentData.resolvedEdge = parentData.primaryEdge;
          } else {
            parentData.resolvedPosition = parentData.secondaryPosition;
            parentData.resolvedEdge = parentData.secondaryEdge;
          }
        }
      }
    }
  }

  @override
  void performLayout() {
    size = constraints.biggest;
    final bounds = padding.deflateRect(Offset.zero & size).pixelSnap(pixelSnap);

    RenderBox? child = firstChild;
    while (child != null) {
      child.layout(BoxConstraints.loose(bounds.size), parentUsesSize: true);
      final _MenuLayoutParentData parentData =
          child.parentData! as _MenuLayoutParentData;
      _resolvePositionIfNeeded(parentData, child.size, bounds);
      assert(parentData.resolvedPosition != null, 'Failed to resolve position');
      assert(parentData.resolvedEdge != null, 'Failed to resolve edge');
      final rect = _getRect(child.size, parentData.resolvedPosition!,
              parentData.resolvedEdge!)
          .positionHorizontalyWithinBounds(bounds)
          .positionVerticalyWithinBounds(bounds);

      parentData.offset = rect.topLeft.pixelSnap(pixelSnap);

      child = childAfter(child);
    }
  }

  @override
  void paint(PaintingContext context, Offset offset) {
    defaultPaint(context, offset);
  }

  @override
  bool hitTestChildren(BoxHitTestResult result, {required Offset position}) {
    return defaultHitTestChildren(result, position: position);
  }
}
