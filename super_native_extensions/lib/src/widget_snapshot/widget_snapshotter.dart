import 'dart:async';

import 'package:flutter/widgets.dart';

import 'widget_snapshot.dart';
import 'widget_snapshotter_internal.dart';

import 'widget_snapshotter_native.dart'
    if (dart.library.js) 'widget_snapshotter_web.dart';

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

class WidgetSnapshotter extends StatefulWidget {
  const WidgetSnapshotter({
    super.key,
    required this.child,
  });

  final Widget child;

  /// Whether widget snapshotter supports snapshotting widgets to ui.Image.
  /// This is true when running native or canvaskit, false for HTML.
  static bool snapshotToImageSupported() => snapshotToImageSupportedInternal();

  @override
  State<StatefulWidget> createState() => WidgetSnapshotterStateImpl();
}

abstract class WidgetSnapshotterState extends State<WidgetSnapshotter> {
  /// Register widget for the key. If `null` [widget] is specified, the default
  /// child widget will be used.
  ///
  /// Registering widget prepares widget for snapshot and allows [getSnapshot]
  /// to provide the snapshot immediately.
  void registerWidget(Object key, Widget? widget);

  /// Unregister widget for given key. If no widget is registered for the key,
  /// this method does nothing.
  void unregisterWidget(Object key);

  /// Returns snapshot for given [location] and [key]. If widget is already
  /// registered for the key, the snapshot will be provided immediately.
  /// Otherwise the [widgetBuilder] will be called to create the widget and
  /// snapshot will be provided as soon as the widget is laid out.
  ///
  /// If [widgetBuilder] returns `null` or `null` widget is registered for given
  /// key, the default child widget will be used as snapshot.
  Future<TargetedWidgetSnapshot?> getSnapshot(
    Offset location,
    Object key,
    ValueGetter<Widget?> widgetBuilder,
  );
}
