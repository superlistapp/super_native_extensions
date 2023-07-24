import 'dart:ui' as ui;
import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';

import '../util.dart';

/// Represents a snapshot of widget.
class WidgetSnapshot {
  /// Creates a new widget snapshot backed by ui.Image.
  /// This is used on platforms that can rasterize widgets to images
  /// (Native + CanvasKit).
  WidgetSnapshot.image(this._image)
      : _renderObject = null,
        _renderObjectBounds = null;

  /// Creates a new widget snapshot backed by a render object.
  /// [bounds] represents the part of the render object that should be
  /// visible.
  WidgetSnapshot.renderObject(this._renderObject, Rect bounds)
      : _image = null,
        _renderObjectBounds = bounds;

  /// Key used to create this snapshot.
  Object? debugKey;

  /// Returns whether this is image backed widget snapshot.
  bool get isImage {
    assert(!_disposed);
    return _image != null;
  }

  /// Returns image representation of this snapshot.
  ui.Image get image {
    assert(!_disposed);
    assert(isImage);
    return _image!;
  }

  /// Returns whether this is render object backed widget snapshot.
  bool get isRenderObject {
    assert(!_disposed);
    return _renderObject != null;
  }

  int _retainCount = 1;

  /// Increases the retain count of this snapshot. This allows using snapshot
  /// in multiple places (i.e. lift & drag having same image).
  WidgetSnapshot retain() {
    _retainCount++;
    return this;
  }

  /// Returns render object representation of this snapshot. Note that this
  /// can only be called per snapshot. After that the render object is free to be disposed.
  /// If you need to use snapshot in multiple places (i.e. lift & drag having same image)
  /// use [retain] to increase the retain count.
  RenderObject? getRenderObject() {
    assert(!_disposed);
    assert(_retainCount > 0);
    --_retainCount;

    if (_retainCount == 0) {
      _onRenderObjectRequested.notify();
    }
    if (_retainCount >= 0) {
      return _renderObject!;
    } else {
      return null;
    }
  }

  bool get debugRenderObjectRequested => _retainCount <= 0;

  /// Returns part of the render object represented by this snapshot.
  Rect get renderObjectBounds {
    assert(!_disposed);
    assert(isRenderObject);
    return _renderObjectBounds!;
  }

  double get pointWidth => pointSize.width;
  double get pointHeight => pointSize.height;

  Listenable get onDisposed => _onDisposed;
  Listenable get onRenderObjectRequested => _onRenderObjectRequested;

  final _onDisposed = SimpleNotifier();
  final _onRenderObjectRequested = SimpleNotifier();

  bool _disposed = false;

  void dispose() {
    assert(!_disposed);
    _onDisposed.notify();
    _onDisposed.dispose();
    _image?.dispose();
    _disposed = true;
  }

  Size get pointSize {
    if (isImage) {
      return Size(
        _image!.width / (_image!.devicePixelRatio ?? 1.0),
        _image!.height / (_image!.devicePixelRatio ?? 1.0),
      );
    } else {
      return _renderObjectBounds!.size;
    }
  }

  @override
  String toString() {
    return 'WidgetSnapshot ${_image != null ? 'image' : 'renderObject'} ${identityHashCode(this)} (Debug Key: $debugKey)';
  }

  final ui.Image? _image;
  final RenderObject? _renderObject;
  final Rect? _renderObjectBounds;
}

/// Image representation of part of user interface.
class TargetedWidgetSnapshot {
  TargetedWidgetSnapshot(this.snapshot, this.rect);

  /// Image to be used as avatar image.
  final WidgetSnapshot snapshot;

  /// Initial position of avatar image (in global coordinates).
  final Rect rect;

  /// Retains the targeted snapshot. See [WidgetSnapshot.retain].
  TargetedWidgetSnapshot retain() {
    snapshot.retain();
    return this;
  }

  void dispose() {
    snapshot.dispose();
  }
}

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
