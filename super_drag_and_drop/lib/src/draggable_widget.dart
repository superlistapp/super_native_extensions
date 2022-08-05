import 'package:flutter/foundation.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
export 'package:super_native_extensions/raw_drag_drop.dart'
    show DropOperation, DragSession;

import 'base_draggable_widget.dart';

typedef DragItemProvider = Future<DragItem?> Function(
    AsyncValueGetter<DragImage> snapshot, DragSession session);

class DragItemWidget extends StatefulWidget {
  const DragItemWidget({
    super.key,
    required this.child,
    required this.dragItem,
    required this.allowedOperations,
    this.canAddItemToExistingSession = false,
  });

  final Widget child;

  /// Callback that can provide drag item for this widget.
  final DragItemProvider dragItem;

  /// Allowed drag operations for this item. If multiple items are being
  /// dragged intersection of all allowed operations will be used.
  final ValueGetter<List<raw.DropOperation>> allowedOperations;

  /// Whether on iOS this widget can contribute item to existing drag session.
  /// If true the item provider should check local data of existing session
  /// to determine if this item already exists in the session. Otherwise
  /// tapping item repeatedly during dragging will result in item being added
  /// multiple times.
  final bool canAddItemToExistingSession;

  @override
  State<StatefulWidget> createState() => DragItemWidgetState();
}

class DragItemWidgetState extends State<DragItemWidget> {
  final repaintBoundary = GlobalKey();

  Future<DragImage> _getSnapshot() async {
    final renderObject = repaintBoundary.currentContext?.findRenderObject()
        as RenderRepaintBoundary;
    final image = await renderObject.toImage(
        pixelRatio: MediaQuery.of(context).devicePixelRatio);
    final transform = renderObject.getTransformTo(null);
    final r =
        Rect.fromLTWH(0, 0, renderObject.size.width, renderObject.size.height);
    final rect = MatrixUtils.transformRect(transform, r);
    return DragImage(image, rect);
  }

  Future<DragItem?> createItem(DragSession session) async {
    return widget.dragItem(_getSnapshot, session);
  }

  Future<List<raw.DropOperation>> getAllowedOperations() async {
    return widget.allowedOperations();
  }

  @override
  Widget build(BuildContext context) {
    return RepaintBoundary(
      key: repaintBoundary,
      child: widget.child,
    );
  }
}

typedef DragItemsProvider = List<DragItemWidgetState> Function(
    BuildContext context);

class DraggableWidget extends StatelessWidget {
  const DraggableWidget({
    super.key,
    required this.child,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    this.dragItems = _defaultDragItemsProvider,
    this.additionalDragItems = _defaultDragItemsProvider,
  });

  final Widget child;
  final HitTestBehavior hitTestBehavior;
  final DragItemsProvider dragItems;
  final DragItemsProvider additionalDragItems;

  static List<DragItemWidgetState> _defaultDragItemsProvider(
      BuildContext context) {
    final state = context.findAncestorStateOfType<DragItemWidgetState>();
    if (state != null) {
      return [state];
    } else {
      throw Exception('SimpleDraggable must be placed inside a DragItemWidget');
    }
  }

  Future<DragConfiguration?> dragConfigurationForItems(
      List<DragItemWidgetState> items, DragSession session) async {
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
      final dragItems = <DragItem>[];
      for (final item in items) {
        final dragItem = await item.createItem(session);
        if (dragItem != null) {
          dragItems.add(dragItem);
        }
      }
      if (dragItems.isNotEmpty) {
        return DragConfiguration(
            items: dragItems, allowedOperations: allowedOperations!);
      }
    }
    return null;
  }

  Future<List<DragItem>?> additionalItems(
      List<DragItemWidgetState> items, DragSession session) async {
    final dragItems = <DragItem>[];
    for (final item in items) {
      if (item.widget.canAddItemToExistingSession) {
        final dragItem = await item.createItem(session);
        if (dragItem != null) {
          dragItems.add(dragItem);
        }
      }
    }
    if (dragItems.isNotEmpty) {
      return dragItems;
    } else {
      return null;
    }
  }

  @override
  Widget build(BuildContext context) {
    return BaseDraggableWidget(
      hitTestBehavior: hitTestBehavior,
      child: child,
      dragConfiguration: (_, session) async {
        final items = dragItems(context);
        return dragConfigurationForItems(items, session);
      },
      additionalItems: (_, session) async {
        final items = additionalDragItems(context);
        return additionalItems(items, session);
      },
    );
  }
}
