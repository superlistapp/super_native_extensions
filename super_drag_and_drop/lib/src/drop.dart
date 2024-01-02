import 'dart:async';
import 'dart:ui' as ui;
import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_clipboard/super_clipboard.dart';

import 'drop_internal.dart';
import 'model.dart';

/// Single item being dropped in a [DropSession].
abstract class DropItem with Diagnosticable {
  /// Returns whether the item contains data for given [DataFormat].
  /// This is a best guess based on format identifier. It is still possible
  /// that [dataReader] will fail to provide the data during drop.
  bool canProvide(DataFormat f);

  @Deprecated('use canProvide instead')
  bool hasValue(DataFormat f) => canProvide(f);

  /// Local data associated with this drop item that can be only read when
  /// dropping within the same application.
  Object? get localData;

  /// [DataReader] that can be used to access drag data for this item.
  /// `dataReader` is always available in `onPerformDrop` event, but may be `null`
  /// during the `onDropOver` event in following cases:
  /// - on mobile and web, `dataReader` is always `null` during `onDropOver`.
  ///   For security reasons the data is only available after user actually
  ///   performed the drop.
  /// - on desktop `dataReader` is gradually populated during `onDropOver` event.
  ///   On some platforms clipboard access can get slow with large amount of items,
  ///   and rather than blocking main thread the items are populated asynchronously.
  DataReader? get dataReader;

  /// Returns list of platform specific format identifier for this item.
  List<PlatformFormat> get platformFormats;
}

/// Allows querying the state of drop session such as the items being dropped
/// and allowed drop operations.
abstract class DropSession with Diagnosticable {
  /// List of items being dropped. List content may change over time for single
  /// session on some platforms, for example iOS allows adding drop items
  /// to existing session.
  ///
  /// The items instances for single [DropSession] are guaranteed to be same
  /// between individual drop events.
  List<DropItem> get items;

  /// Invoked when this drop session is disposed (could be either after drop
  /// is performed or cancelled).
  Listenable get onDisposed;

  /// Drop operations that the drag source allows.
  Set<DropOperation> get allowedOperations;
}

/// Position for drop event.
class DropPosition {
  DropPosition({
    required this.local,
    required this.global,
  });

  /// Drop position in local coordinates of [DropRegion] or [DropMonitor] widget.
  final Offset local;

  /// Drop position in global coordinates (within the Flutter view).
  final Offset global;

  static DropPosition forRenderObject(
      Offset globalPosition, RenderObject object) {
    final transform = object.getTransformTo(null);
    transform.invert();
    return DropPosition(
      local: MatrixUtils.transformPoint(transform, globalPosition),
      global: globalPosition,
    );
  }

  DropPosition transformedToRenderObject(RenderObject object) =>
      DropPosition.forRenderObject(global, object);
}

/// Base drop event containing only [DropSession] with no additional
/// information.
class DropEvent {
  /// Drop session associated with this event.
  final DropSession session;

  DropEvent({
    required this.session,
  });
}

/// Drop event sent when dragging over [DropRegion] before the drop.
class DropOverEvent {
  /// Drop session associated with this event.
  final DropSession session;

  /// Position of the drop event.
  final DropPosition position;

  DropOverEvent({
    required this.session,
    required this.position,
  });
}

/// Drop event sent when user performs drop.
class PerformDropEvent {
  /// Drop session associated with this event.
  final DropSession session;

  /// Position of the drop event.
  final DropPosition position;

  /// Accepted operation from last [DropOverEvent].
  final DropOperation acceptedOperation;

  PerformDropEvent({
    required this.session,
    required this.position,
    required this.acceptedOperation,
  });
}

/// Allows customizing drop animation on macOS and iOS.
typedef OnGetDropItemPreview = FutureOr<DropItemPreview?> Function(
  DropSession session,
  DropItemPreviewRequest request,
);

/// Type of render object produced by [DropRegion] and [RenderDropMonitor].
enum RenderObjectType {
  box,
  sliver,
}

/// Widget to which data can be dropped.
class DropRegion extends SingleChildRenderObjectWidget {
  const DropRegion({
    super.key,
    required super.child,
    required this.formats,
    required this.onDropOver,
    required this.onPerformDrop,
    this.onDropEnter,
    this.onDropLeave,
    this.onDropEnded,
    this.onGetDropItemPreview,
    this.renderObjectType = RenderObjectType.box,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
  });

  final RenderObjectType renderObjectType;

  final HitTestBehavior hitTestBehavior;

  /// List of [DataFormat]s this [DropRegion] is interested in. May be empty
  /// if region only wants to accept local drag sessions.
  final List<DataFormat> formats;

  /// Invoked when dragging happens over this region. Implementation should
  /// inspect the drag session from event and return a drop operation
  /// that it can support (or [DropOperation.none]).
  final FutureOr<DropOperation> Function(DropOverEvent) onDropOver;

  /// Invoked when user performs drop on this region.
  final Future<void> Function(PerformDropEvent) onPerformDrop;

  /// Invoked once after inactive region accepts the drop.
  final void Function(DropEvent)? onDropEnter;

  /// Invoked when dragging leaves the region.
  final void Function(DropEvent)? onDropLeave;

  /// Invoked when drop session has finished.
  final void Function(DropEvent)? onDropEnded;

  /// Allows customizing drop animation on macOS and iOS.
  final OnGetDropItemPreview? onGetDropItemPreview;

  @override
  RenderObject createRenderObject(BuildContext context) {
    switch (renderObjectType) {
      case RenderObjectType.box:
        return RenderDropRegionBox(
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
      case RenderObjectType.sliver:
        return RenderDropRegionSliver(
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
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as RenderDropRegion;
    if (renderObject_ is RenderProxyBoxWithHitTestBehavior) {
      (renderObject_ as RenderProxyBoxWithHitTestBehavior).behavior =
          hitTestBehavior;
    }
    renderObject_.updateFormats(formats);
    renderObject_.onDropOver = onDropOver;
    renderObject_.onDropLeave = onDropLeave;
    renderObject_.onPerformDrop = onPerformDrop;
    renderObject_.onDropEnded = onDropEnded;
    renderObject_.onGetDropItemPreview = onGetDropItemPreview;
  }
}

/// Event sent to [DropMonitor]s when dragging anywhere over the application.
class MonitorDropOverEvent extends DropOverEvent {
  /// Whether dragging happens over the receiver [DropMonitor].
  final bool isInside;

  MonitorDropOverEvent({
    required super.session,
    required super.position,
    required this.isInside,
  });
}

/// Widget that can monitor drag events over the entire Flutter view.
///
/// Unlike [DropRegion] this widget can not accept drops, but it gets
/// notification of all drop events, including those that are not happening
/// immediately over the region.
class DropMonitor extends SingleChildRenderObjectWidget {
  const DropMonitor({
    super.key,
    super.child,
    this.renderObjectType = RenderObjectType.box,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    required this.formats,
    this.onDropOver,
    this.onDropLeave,
    this.onDropEnded,
  });

  final RenderObjectType renderObjectType;
  final HitTestBehavior hitTestBehavior;
  final List<DataFormat> formats;

  /// Invoked when drop is happening anywhere over the Flutter view.
  /// `isInside` field of the event will be `true` if drop is happening
  /// over this [DropMonitor].
  final void Function(MonitorDropOverEvent)? onDropOver;

  /// Invoked when drop leaves the Flutter view (not just this monitor).
  final void Function(DropEvent)? onDropLeave;

  /// Invoked when drop session ends.
  final void Function(DropEvent)? onDropEnded;

  @override
  RenderObject createRenderObject(BuildContext context) {
    switch (renderObjectType) {
      case RenderObjectType.box:
        return RenderDropMonitorBox(
          behavior: hitTestBehavior,
          formats: formats,
          onDropOver: onDropOver,
          onDropLeave: onDropLeave,
          onDropEnded: onDropEnded,
        );
      case RenderObjectType.sliver:
        return RenderDropMonitorSliver(
          formats: formats,
          onDropOver: onDropOver,
          onDropLeave: onDropLeave,
          onDropEnded: onDropEnded,
        );
    }
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject) {
    final renderObject_ = renderObject as RenderDropMonitor;
    if (renderObject_ is RenderProxyBoxWithHitTestBehavior) {
      (renderObject_ as RenderProxyBoxWithHitTestBehavior).behavior =
          hitTestBehavior;
    }
    renderObject_.updateFormats(formats);
    renderObject_.onDropOver = onDropOver;
    renderObject_.onDropLeave = onDropLeave;
    renderObject_.onDropEnded = onDropEnded;
  }
}

/// Requests for providing target preview for item being dropped.
abstract class DropItemPreviewRequest {
  /// Item for which the preview is requested.
  DropItem get item;

  /// Size of dragging image.
  Size get size;

  /// Default delay before the item preview starts fading out.
  Duration get fadeOutDelay;

  /// Default duration of item fade out.
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

  /// Destination (in global coordinates) to where the item should land.
  final Rect destinationRect;

  /// Destination image to which the drag image will morph. If not provided,
  /// drag image will be used.
  final ui.Image? destinationImage;

  /// Override fade out delay (iOS only)
  final Duration? fadeOutDelay;

  /// Override fade out duration (iOS only)
  final Duration? fadeOutDuration;
}
