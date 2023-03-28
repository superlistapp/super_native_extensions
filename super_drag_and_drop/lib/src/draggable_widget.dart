import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
import 'package:super_native_extensions/widgets.dart';
export 'package:super_native_extensions/raw_drag_drop.dart'
    show DropOperation, DragSession;

import 'drag_configuration.dart';
import 'base_draggable_widget.dart';

class DragItemRequest {
  DragItemRequest({
    required this.location,
    required this.session,
  });

  /// Drag location in global coordinates.
  final Offset location;

  /// Current drag session.
  final raw.DragSession session;
}

typedef DragItemProvider = FutureOr<DragItem?> Function(DragItemRequest);

/// Widget that provides [DragItem] for a [DraggableWidget].
///
/// Example usage
/// ```dart
/// DragItemWidget(
///   dragItemProvider: (request) async {
///     // DragItem represents the content bein dragged.
///     final item = DragItem(
///       // snapshot() will return image snapshot of the DragItemWidget.
///       // You can use any other drag image if your wish.
///       image: request.dragImage(),
///       // This data is only accessible when dropping within same
///       // application. (optional)
///       localData: {'x': 3, 'y': 4},
///     );
///     // Add data for this item that other applications can read
///     // on drop. (optional)
///     item.add(Formats.plainText('Plain Text Data'));
///     item.add(
///         Formats.htmlText.lazy(() => '<b>HTML generated on demand</b>'));
///     return item;
///   },
///   allowedOperations: () => [DropOperation.copy],
///   // DraggableWidget represents the actual user-draggable area. It looks
///   // for parent DragItemWidget in widget hierarchy to provide the DragItem.
///   child: const DraggableWidget(
///     child: Text('This widget is draggable'),
///   ),
/// );
/// ```
class DragItemWidget extends StatefulWidget {
  const DragItemWidget({
    super.key,
    required this.child,
    required this.dragItemProvider,
    required this.allowedOperations,
    this.canAddItemToExistingSession = false,
  });

  final Widget child;

  /// Callback that provides drag item for this widget. If `null` is returned
  /// the drag will not start.
  final DragItemProvider dragItemProvider;

  /// Allowed drag operations for this item. If multiple items are being
  /// dragged intersection of all allowed operations will be used.
  final ValueGetter<List<raw.DropOperation>> allowedOperations;

  /// Whether on iOS this widget can contribute item to existing drag session.
  /// If true the item provider should check local data of drag session
  /// to determine if this item already exists in the session. Otherwise
  /// tapping item repeatedly during dragging will result in item being added
  /// multiple times.
  final bool canAddItemToExistingSession;

  @override
  State<StatefulWidget> createState() => DragItemWidgetState();
}

class DragItemWidgetState extends State<DragItemWidget> {
  Future<DragImage?> _getSnapshot(Offset location) async {
    final snapshotter = Snapshotter.of(_innerContext!)!;
    final dragSnapshot =
        await snapshotter.getSnapshot(location, SnapshotType.drag);

    raw.TargetedImage? liftSnapshot;
    if (defaultTargetPlatform == TargetPlatform.iOS) {
      liftSnapshot = await snapshotter.getSnapshot(location, SnapshotType.lift);
    }

    final snapshot = dragSnapshot ??
        liftSnapshot ??
        await snapshotter.getSnapshot(location, null);

    if (snapshot == null) {
      return null;
    }

    return DragImage(image: snapshot, liftImage: liftSnapshot);
  }

  Future<DragConfigurationItem?> createItem(
      Offset location, raw.DragSession session) async {
    final request = DragItemRequest(
      location: location,
      session: session,
    );

    final item = await widget.dragItemProvider(request);
    if (item != null) {
      final image = await _getSnapshot(location);
      if (image != null) {
        return DragConfigurationItem(
          item: item,
          image: image,
        );
      }
    }
    return null;
  }

  Future<List<raw.DropOperation>> getAllowedOperations() async {
    return widget.allowedOperations();
  }

  BuildContext? _innerContext;

  @override
  Widget build(BuildContext context) {
    return FallbackSnapshotWidget(
      child: Builder(builder: (context) {
        _innerContext = context;
        return widget.child;
      }),
    );
  }
}

typedef DragItemsProvider = List<DragItemWidgetState> Function(
    BuildContext context);

typedef OnDragConfiguration = FutureOr<DragConfiguration?> Function(
    DragConfiguration configuration, raw.DragSession session);

typedef OnAdditonalItems = FutureOr<List<DragConfigurationItem>?> Function(
    List<DragConfigurationItem> items, raw.DragSession session);

/// Widget that represents user-draggable area.

/// By default the widget will look for [DragItemWidget] in parent widget
/// hierarchy in order to provide data for the drag session.
class DraggableWidget extends StatelessWidget {
  const DraggableWidget({
    super.key,
    required this.child,
    this.onDragConfiguration,
    this.onAdditonalItems,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    this.isLocationDraggable = _defaultIsLocationDraggable,
    this.dragItemsProvider = _defaultDragItemsProvider,
    this.additionalDragItemsProvider = _defaultDragItemsProvider,
  });

  final Widget child;
  final HitTestBehavior hitTestBehavior;

  /// Should return true if the offset is considered draggable.
  /// The offset is in global coordinates but restricted to area covered
  /// by the Widget.
  final LocationIsDraggable isLocationDraggable;

  /// Allows post-processing initial drag configuration.
  final OnDragConfiguration? onDragConfiguration;

  /// Allows post-processing additional items added to drag session.
  final OnAdditonalItems? onAdditonalItems;

  final DragItemsProvider dragItemsProvider;
  final DragItemsProvider additionalDragItemsProvider;

  static bool _defaultIsLocationDraggable(Offset position) => true;

  static List<DragItemWidgetState> _defaultDragItemsProvider(
      BuildContext context) {
    final state = context.findAncestorStateOfType<DragItemWidgetState>();
    if (state != null) {
      return [state];
    } else {
      throw Exception('DraggableWidget must be placed inside a DragItemWidget');
    }
  }

  Future<DragConfiguration?> dragConfigurationForItems(
      List<DragItemWidgetState> items,
      Offset location,
      raw.DragSession session) async {
    List<raw.DropOperation>? allowedOperations;
    for (final item in items) {
      if (allowedOperations == null) {
        allowedOperations = List.from(await item.getAllowedOperations());
      } else {
        final itemOperations = await item.getAllowedOperations();
        allowedOperations
            .retainWhere((element) => itemOperations.contains(element));
      }
    }

    if (allowedOperations?.isNotEmpty == true) {
      final dragItems = <DragConfigurationItem>[];
      for (final item in items) {
        final dragItem = await item.createItem(location, session);
        if (dragItem != null) {
          dragItems.add(dragItem);
        }
      }
      if (dragItems.isNotEmpty) {
        final configuration = DragConfiguration(
          items: dragItems,
          allowedOperations: allowedOperations!,
        );
        if (onDragConfiguration != null) {
          return onDragConfiguration!(configuration, session);
        } else {
          return configuration;
        }
      }
    }
    return null;
  }

  Future<List<DragConfigurationItem>?> additionalItems(
      List<DragItemWidgetState> items,
      Offset location,
      raw.DragSession session) async {
    final dragItems = <DragConfigurationItem>[];
    for (final item in items) {
      if (item.widget.canAddItemToExistingSession) {
        final dragItem = await item.createItem(location, session);
        if (dragItem != null) {
          dragItems.add(dragItem);
        }
      }
    }
    if (dragItems.isNotEmpty) {
      if (onAdditonalItems != null) {
        return onAdditonalItems!(dragItems, session);
      } else {
        return dragItems;
      }
    } else {
      return null;
    }
  }

  @override
  Widget build(BuildContext context) {
    return BaseDraggableWidget(
      isLocationDraggable: isLocationDraggable,
      hitTestBehavior: hitTestBehavior,
      child: child,
      dragConfiguration: (location, session) async {
        final items = dragItemsProvider(context);
        return dragConfigurationForItems(items, location, session);
      },
      additionalItems: (location, session) async {
        final items = additionalDragItemsProvider(context);
        return additionalItems(items, location, session);
      },
    );
  }
}
