import 'dart:ui' as ui;
import 'package:flutter/widgets.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'drop_internal.dart';

abstract class DropItem {
  List<String> get formats;
  Object? get localData;
  DataReader? get dataReader;
}

abstract class DropSession {
  List<DropItem> get items;
  Listenable get onDisposed;
  Set<raw.DropOperation> get allowedOperations;
}

typedef OnDropOver = Future<raw.DropOperation> Function(
  DropSession session,
  Offset position,
);

typedef OnDropLeave = void Function(DropSession session);

typedef OnDropEnded = void Function(DropSession session);

typedef OnPerformDrop = Future<void> Function(
  DropSession session,
  Offset position,
  raw.DropOperation acceptedOperation,
);

/// Allows customizing drop animation on macOS and iOS.
typedef OnGetDropItemPreview = Future<DropItemPreview?> Function(
  DropSession session,
  DropItemPreviewRequest request,
);

class RawDropRegion extends SingleChildRenderObjectWidget {
  const RawDropRegion({
    super.key,
    required super.child,
    required this.formats,
    required this.onDropOver,
    required this.onDropLeave,
    required this.onPerformDrop,
    this.onDropEnded = _defaultOnDropEnded,
    this.onGetDropItemPreview = _defaultPreview,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
  });

  final HitTestBehavior hitTestBehavior;

  final List<EncodableDataFormat> formats;
  final OnDropOver onDropOver;
  final OnDropLeave onDropLeave;
  final OnPerformDrop onPerformDrop;
  final OnDropEnded onDropEnded;
  final OnGetDropItemPreview onGetDropItemPreview;

  static void _defaultOnDropEnded(DropSession sessions) {}

  static Future<DropItemPreview?> _defaultPreview(
      DropSession session, DropItemPreviewRequest req) async {
    return null;
  }

  @override
  RenderObject createRenderObject(BuildContext context) {
    return RenderRawDropRegion(
      behavior: hitTestBehavior,
      formats: formats,
      onDropOver: onDropOver,
      onDropLeave: onDropLeave,
      onPerformDrop: onPerformDrop,
      onDropEnded: onDropEnded,
      onGetDropItemPreview: onGetDropItemPreview,
      devicePixelRatio: MediaQuery.of(context).devicePixelRatio,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as RenderRawDropRegion;
    renderObject_.behavior = hitTestBehavior;
    renderObject_.formatRegistration.dispose();
    renderObject_.formatRegistration =
        DropFormatRegistry.instance.registerFormats(formats);
    renderObject_.onDropOver = onDropOver;
    renderObject_.onDropLeave = onDropLeave;
    renderObject_.onPerformDrop = onPerformDrop;
    renderObject_.onDropEnded = onDropEnded;
    renderObject_.onGetDropItemPreview = onGetDropItemPreview;
    renderObject_.devicePixelRatio = MediaQuery.of(context).devicePixelRatio;
  }
}

typedef OnMonitorDropOver = void Function(
    DropSession session, Offset position, bool isInside);

class DropMonitor extends SingleChildRenderObjectWidget {
  const DropMonitor({
    super.key,
    super.child,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    required this.formats,
    required this.onDropOver,
    required this.onDropLeave,
    this.onDropEnded = _defaultOnDropEnded,
  });

  final HitTestBehavior hitTestBehavior;
  final List<EncodableDataFormat> formats;
  final OnMonitorDropOver onDropOver;
  final OnDropLeave onDropLeave;
  final OnDropEnded onDropEnded;

  static void _defaultOnDropEnded(DropSession sessions) {}

  @override
  RenderObject createRenderObject(BuildContext context) {
    return RenderDropMonitor(
      behavior: hitTestBehavior,
      formats: formats,
      onDropOver: onDropOver,
      onDropLeave: onDropLeave,
      onDropEnded: onDropEnded,
    );
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as RenderDropMonitor;
    renderObject_.behavior = hitTestBehavior;
    renderObject_.formatRegistration.dispose();
    renderObject_.formatRegistration =
        DropFormatRegistry.instance.registerFormats(formats);
    renderObject_.onDropOver = onDropOver;
    renderObject_.onDropLeave = onDropLeave;
    renderObject.onDropEnded = onDropEnded;
  }
}

//

abstract class DropItemPreviewRequest {
  /// Item for which the preview is requested.
  DropItem get item;

  /// Size of dragging image;
  Size get size;

  /// Default delay before the item preview starts fading out
  Duration get fadeOutDelay;

  /// Default duration of item fade out
  Duration get fadeOutDuration;
}

class DropItemPreview {
  DropItemPreview({
    required this.destinationRect,
    this.destinationImage,

    /// iOS only
    this.fadeOutDelay,

    /// iOS only
    this.fadeOutDuration,
  });

  /// Destination (in global cooridantes) to where the item should land.
  final Rect destinationRect;

  /// Destination image to which the drag image will morph. If not provided,
  /// drag image will be used.
  final ui.Image? destinationImage;

  /// Override fade out delay
  final Duration? fadeOutDelay;

  /// Override fade out duration
  final Duration? fadeOutDuration;
}
