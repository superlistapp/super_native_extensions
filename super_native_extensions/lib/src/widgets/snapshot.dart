import 'dart:async';

import 'package:flutter/widgets.dart';

import '../api_model.dart';
import 'snapshot_internal.dart';

enum SnapshotType {
  /// Snapshot used for lift animation on iOS.
  lift,

  /// Snapshot used during dragging.
  drag,
}

typedef Translation = Offset Function(
  /// Snapshot rectangle in local coordinates.
  Rect rect,

  /// Drag position within the rectangle.
  Offset dragPosition,
);

/// Wrapper widget that allows customizing snapshot settings.
class SnapshotSettings extends StatefulWidget {
  const SnapshotSettings({
    super.key,
    required this.child,
    this.constraintsTransform,
    this.translation,
  });

  final Widget child;

  /// Allows to transform constraints for snapshot widget. The resulting
  /// constraints may exceed parent constraints without causing an error.
  final BoxConstraintsTransform? constraintsTransform;

  /// Allows to transform snapshot location.
  final Translation? translation;

  @override
  State<SnapshotSettings> createState() => SnapshotSettingsState();
}

typedef SnapshotBuilder = Widget? Function(
  BuildContext context,

  /// Original child widget of [CustomSnapshot].
  Widget child,

  /// Type of snapshot currently being built.
  SnapshotType type,
);

/// Widget that provides customized dragging snapshots from widgets created by
/// by [snapshotBuilder] the function.
///
/// To customize snapshot geometry and position you can wrap the snapshot with
/// [SnapshotSettings] widget.
///
/// If you want to create completely custom dragging snapshot images from
/// scratch you can use [RawCustomSnapshotWidget] instead.
class CustomSnapshotWidget extends StatefulWidget {
  const CustomSnapshotWidget({
    super.key,
    required this.child,
    required this.snapshotBuilder,
  });

  final Widget child;

  /// Builder that creates the widget that will be used as a snapshot for
  /// provided [SnapshotType]s.
  final SnapshotBuilder snapshotBuilder;

  @override
  State<CustomSnapshotWidget> createState() => CustomSnapshotWidgetState();
}

/// Widget that can be used to generate completely custom drag and drop
/// snapshots.
class RawCustomSnapshotWidget extends StatefulWidget {
  const RawCustomSnapshotWidget({
    super.key,
    required this.onGetSnapshot,
    required this.onPrepare,
    required this.child,
    required this.onUnprepare,
  });

  /// This will be called when snapshot is requested.
  /// `type` will be `null` when snapshot is requested as fallback in case
  /// custom snapshot for particular type was not available.
  final Future<TargetedImage?> Function(Offset location, SnapshotType? type)
      onGetSnapshot;

  /// This will be called before any snapshot is requested.
  /// Implementation can use this to prepare snapshots for given types in order
  /// to avoid delays when snapshot is requested.
  final void Function(Set<SnapshotType> types) onPrepare;

  /// After this call it is unlikely for a snapshot to be requested.
  final void Function() onUnprepare;

  final Widget child;

  @override
  State<StatefulWidget> createState() => RawCustomSnapshotWidgetState();
}

//
//
//

abstract class Snapshotter {
  static Snapshotter? of(BuildContext context) {
    final real =
        context.findAncestorStateOfType<RawCustomSnapshotWidgetState>();
    if (real != null) {
      return real;
    } else {
      return context.findAncestorStateOfType<FallbackSnapshotWidgetState>();
    }
  }

  void prepare(Set<SnapshotType> types);

  void unprepare();

  Future<TargetedImage?> getSnapshot(Offset location, SnapshotType? type);
}

class FallbackSnapshotWidget extends StatefulWidget {
  const FallbackSnapshotWidget({
    super.key,
    required this.child,
  });

  final Widget child;

  @override
  State<FallbackSnapshotWidget> createState() => FallbackSnapshotWidgetState();
}
