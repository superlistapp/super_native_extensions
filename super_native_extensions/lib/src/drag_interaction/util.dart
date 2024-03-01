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
    final animation = SimpleAnimation._();

    final ticker = Ticker((elapsed) {
      if (duration.inMilliseconds == 0 || elapsed > duration) {
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

  SimpleAnimation._();

  void cancel() {
    _ticker.stop();
    _ticker.dispose();
  }

  late Ticker _ticker;
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

  /// Best effort to move the rect into the bounds without shrinking it.
  Rect moveIntoRect(Rect bounds) {
    final x = left < bounds.left
        ? bounds.left
        : right > bounds.right
            ? bounds.right - width
            : left;
    final y = top < bounds.top
        ? bounds.top
        : bottom > bounds.bottom
            ? bounds.bottom - height
            : top;
    return Rect.fromLTWH(x, y, width, height);
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
