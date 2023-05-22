import 'dart:math' as math;

import 'package:flutter/material.dart';
import 'package:flutter/rendering.dart';

class GroupIntrinsicWidthContainer extends SingleChildRenderObjectWidget {
  const GroupIntrinsicWidthContainer({
    super.key,
    required super.child,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderGroupChildIntrinsicWidthContainer();
  }
}

class _RenderGroupChildIntrinsicWidthContainer extends RenderProxyBox {
  final _children = <_RenderGroupIntrinsicWidth>{};

  VoidCallback addChild(_RenderGroupIntrinsicWidth child) {
    // Every child can affect other children so we need to mark them all as
    // needing layout.
    for (final c in _children) {
      c.markNeedsLayout();
    }
    _children.add(child);
    return () {
      _children.remove(child);
      for (final c in _children) {
        c.markNeedsLayout();
      }
    };
  }

  double? _cachedIntrinsicWidth;

  /// Returns max intrinsic width for the entire group.
  double getGroupIntrinsicWidth() {
    if (_cachedIntrinsicWidth == null) {
      _cachedIntrinsicWidth = 0;
      for (final child in _children) {
        _cachedIntrinsicWidth = math.max(_cachedIntrinsicWidth!,
            child.originalMaxIntrinsicWidth(double.infinity));
      }
    }
    return _cachedIntrinsicWidth!;
  }

  @override
  void performLayout() {
    super.performLayout();
    _cachedIntrinsicWidth = null;
  }
}

class GroupIntrinsicWidth extends SingleChildRenderObjectWidget {
  const GroupIntrinsicWidth({
    super.key,
    required super.child,
  });

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderGroupIntrinsicWidth();
  }
}

class _RenderGroupIntrinsicWidth extends RenderProxyBox {
  _RenderGroupChildIntrinsicWidthContainer? _getContainer() {
    var parent = this.parent;
    while (parent != null) {
      if (parent is _RenderGroupChildIntrinsicWidthContainer) {
        return parent;
      }
      parent = parent.parent;
    }
    return null;
  }

  VoidCallback? _removeFromParent;

  @override
  void attach(covariant PipelineOwner owner) {
    super.attach(owner);
    _removeFromParent = _getContainer()!.addChild(this);
  }

  @override
  void detach() {
    super.detach();
    _removeFromParent?.call();
  }

  double originalMaxIntrinsicWidth(double height) {
    return super.computeMaxIntrinsicWidth(height);
  }

  @override
  double computeMaxIntrinsicWidth(double height) {
    return _getContainer()!.getGroupIntrinsicWidth();
  }

  @override
  void performLayout() {
    final constraints = BoxConstraints.tightFor(
        width: _getContainer()!.getGroupIntrinsicWidth());
    child!.layout(constraints, parentUsesSize: true);
    size = child!.size;
  }
}
