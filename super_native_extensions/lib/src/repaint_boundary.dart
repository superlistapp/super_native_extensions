import 'dart:ui' as ui;
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';

/// Fork for [RepaintBoundary] that allows toImageSync with custom bounds.
class BetterRepaintBoundary extends SingleChildRenderObjectWidget {
  /// Creates a widget that isolates repaints.
  const BetterRepaintBoundary({super.key, super.child});

  @override
  RenderBetterRepaintBoundary createRenderObject(BuildContext context) =>
      RenderBetterRepaintBoundary();
}

class RenderBetterRepaintBoundary extends RenderProxyBox {
  @override
  bool get isRepaintBoundary => true;

  ui.Image toImageSync({
    Rect? bounds,
    required double pixelRatio,
  }) {
    final OffsetLayer offsetLayer = layer! as OffsetLayer;
    return offsetLayer.toImageSync(bounds ?? Offset.zero & size,
        pixelRatio: pixelRatio);
  }

  Future<ui.Image> toImage({
    Rect? bounds,
    required double pixelRatio,
  }) {
    final OffsetLayer offsetLayer = layer! as OffsetLayer;
    return offsetLayer.toImage(bounds ?? Offset.zero & size,
        pixelRatio: pixelRatio);
  }
}
