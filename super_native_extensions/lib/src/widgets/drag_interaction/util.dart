import 'dart:ui' as ui;
import 'dart:math' as math;

import 'package:flutter/rendering.dart';
import 'package:flutter/scheduler.dart';
import 'package:flutter/widgets.dart';

class SimpleAnimation {
  static SimpleAnimation animate(
    Duration duration,
    Function(double value) callback, {
    VoidCallback? onEnd,
  }) {
    final animation = SimpleAnimation();

    final ticker = Ticker((elapsed) {
      if (elapsed > duration) {
        callback(1.0);
        animation.cancel();
        onEnd?.call();
      } else {
        callback(elapsed.inMilliseconds / duration.inMilliseconds);
      }
    });

    ticker.start();
    animation._ticker = ticker;
    return animation;
  }

  void cancel() {
    _ticker.stop();
    _ticker.dispose();
  }

  late Ticker _ticker;
}

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

  ui.Image toImageSync(
    Rect bounds, {
    required double pixelRatio,
  }) {
    assert(!debugNeedsPaint);
    final OffsetLayer offsetLayer = layer! as OffsetLayer;
    return offsetLayer.toImageSync(bounds, pixelRatio: pixelRatio);
  }

  Future<ui.Image> toImage(
    Rect bounds, {
    required double pixelRatio,
  }) {
    assert(!debugNeedsPaint);
    final OffsetLayer offsetLayer = layer! as OffsetLayer;
    return offsetLayer.toImage(bounds, pixelRatio: pixelRatio);
  }
}

extension SizeExt on Size {
  Size fitInto(Size size) {
    final scale = math
        .min(size.width / width, size.height / height) //
        .clamp(0.0, 1.0);
    return Size(width * scale, height * scale);
  }

  Size inflate(double value) {
    return Size(width + value, height + value);
  }
}

extension RectExt on Rect {
  Rect inflateBy(double factor) {
    return Rect.fromCenter(
        center: center, width: width * factor, height: height * factor);
  }

  Rect copyWith({
    double? left,
    double? top,
    double? width,
    double? height,
  }) {
    return Rect.fromLTWH(
      left ?? this.left,
      top ?? this.top,
      width ?? this.width,
      height ?? this.height,
    );
  }

  Rect fitIntoRect(Rect bounds) {
    final scale =
        math.min(bounds.width / width, bounds.height / height).clamp(0.0, 1.0);
    final newSize = size * scale;
    final x = left < bounds.left
        ? bounds.left
        : right > bounds.right
            ? bounds.right - newSize.width
            : left;
    final y = top < bounds.top
        ? bounds.top
        : bottom > bounds.bottom
            ? bounds.bottom - newSize.height
            : top;
    return Rect.fromLTWH(x, y, newSize.width, newSize.height);
  }
}

extension EdgeInsetsExt on EdgeInsets {
  EdgeInsets atLeast(EdgeInsets insets) => EdgeInsets.only(
        left: left < insets.left ? insets.left : left,
        top: top < insets.top ? insets.top : top,
        right: right < insets.right ? insets.right : right,
        bottom: bottom < insets.bottom ? insets.bottom : bottom,
      );
}
