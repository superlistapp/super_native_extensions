import 'dart:async';
import 'dart:ui' as ui;
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'drop_internal.dart';

abstract class DropItem with Diagnosticable {
  bool hasValue(DataFormat f);
  Object? get localData;

  DataReader? get dataReader;

  List<PlatformFormat> get platformFormats;
}

abstract class DropSession with Diagnosticable {
  List<DropItem> get items;
  Listenable get onDisposed;
  Set<raw.DropOperation> get allowedOperations;
}

typedef OnDropOver = FutureOr<raw.DropOperation> Function(
  DropSession session,
  Offset position,
);

typedef OnDropEnter = void Function(DropSession session);
typedef OnDropLeave = void Function(DropSession session);
typedef OnDropEnded = void Function(DropSession session);

typedef OnPerformDrop = FutureOr<void> Function(
  DropSession session,
  Offset position,
  raw.DropOperation acceptedOperation,
);

/// Allows customizing drop animation on macOS and iOS.
typedef OnGetDropItemPreview = FutureOr<DropItemPreview?> Function(
  DropSession session,
  DropItemPreviewRequest request,
);

class DropRegion extends SingleChildRenderObjectWidget {
  const DropRegion({
    super.key,
    required super.child,
    required this.formats,
    required this.onDropOver,
    this.onDropEnter,
    this.onDropLeave,
    required this.onPerformDrop,
    this.onDropEnded,
    this.onGetDropItemPreview,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
  });

  final HitTestBehavior hitTestBehavior;

  final List<DataFormat> formats;
  final OnDropOver onDropOver;
  final OnDropEnter? onDropEnter;
  final OnDropLeave? onDropLeave;
  final OnPerformDrop onPerformDrop;
  final OnDropEnded? onDropEnded;
  final OnGetDropItemPreview? onGetDropItemPreview;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return RenderDropRegion(
      behavior: hitTestBehavior,
      formats: formats,
      onDropOver: onDropOver,
      onDropEnter: onDropEnter,
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
    final renderObject_ = renderObject as RenderDropRegion;
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
  final List<DataFormat> formats;
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
    renderObject_.onDropEnded = onDropEnded;
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

  /// Destination (in global coordintes) to where the item should land.
  final Rect destinationRect;

  /// Destination image to which the drag image will morph. If not provided,
  /// drag image will be used.
  final ui.Image? destinationImage;

  /// Override fade out delay (iOS only)
  final Duration? fadeOutDelay;

  /// Override fade out duration (iOS only)
  final Duration? fadeOutDuration;
}
