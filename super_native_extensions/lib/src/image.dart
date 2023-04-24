import 'dart:ui' as ui;

final _devicePixelRatio = Expando('devicePixelRatio');

/// Extension on image that allows to store device pixel ratio alongside the
/// image itself.
extension DevicePixelRatio on ui.Image {
  /// Device pixel ratio of the image or null if not set.
  double? get devicePixelRatio => _devicePixelRatio[this] as double?;

  set devicePixelRatio(double? value) {
    _devicePixelRatio[this] = value;
  }

  /// Returns the width of image in resolution independent points.
  double get pointWidth => width / (devicePixelRatio ?? 1.0);

  /// Returns the height of image in resolution independent points.
  double get pointHeight => height / (devicePixelRatio ?? 1.0);

  ui.Size get pointSize {
    final devicePixelRatio = this.devicePixelRatio ?? 1.0;
    return ui.Size(width / devicePixelRatio, height / devicePixelRatio);
  }
}
