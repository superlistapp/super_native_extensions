import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'drag_internal.dart';
import 'draggable_widget.dart';
import 'drag_configuration.dart';

typedef LocationIsDraggable = bool Function(Offset position);
typedef DragConfigurationProvider = Future<DragConfiguration?> Function(
    Offset position, DragSession session);
typedef AdditionalItemsProvider = Future<List<DragItem>?> Function(
    Offset position, DragSession session);

/// This is the most basic draggable widget. It gives you complete control
/// over creating of the drag session.
///
/// In most cases you will want to use [DraggableWidget] instead.
class BaseDraggableWidget extends StatelessWidget {
  const BaseDraggableWidget({
    super.key,
    required this.child,
    required this.dragConfiguration,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    this.isLocationDraggable = _defaultIsLocationDraggable,
    this.additionalItems = _defaultAdditionalItems,
  });

  final Widget child;

  final HitTestBehavior hitTestBehavior;

  /// Returns drag configuration for the given offset and session.
  final DragConfigurationProvider dragConfiguration;

  /// Should return true if the offset is considered draggable.
  /// The offset is in global coordinates but restricted to area covered
  /// by the Widget.
  final LocationIsDraggable isLocationDraggable;

  /// On iOS this method is called when user taps draggable widget
  /// during existing drag sessions. It can be used to provide additional
  /// dragging item for current session.
  final AdditionalItemsProvider additionalItems;

  static Future<List<DragItem>?> _defaultAdditionalItems(
      Offset position, DragSession session) async {
    return null;
  }

  static bool _defaultIsLocationDraggable(Offset position) => true;

  @override
  Widget build(BuildContext context) {
    var child = this.child;
    if (defaultTargetPlatform == TargetPlatform.iOS && !kIsWeb) {
      // on iOS the drag detector is not used to start drag (dragging is driven
      // from iOS UI interaction). The delayed recognizer is needed because
      // otherwise the scroll activity disables user interaction too early
      // and the hit test fails.
      child = DummyDragDetector(child: child);
    } else if (defaultTargetPlatform == TargetPlatform.android ||
        defaultTargetPlatform == TargetPlatform.iOS) {
      child = MobileDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    } else {
      child = DesktopDragDetector(
          dragConfiguration: dragConfiguration, child: child);
    }
    return BaseDraggableRenderWidget(
      hitTestBehavior: hitTestBehavior,
      getDragConfiguration: dragConfiguration,
      isLocationDraggable: isLocationDraggable,
      additionalItems: additionalItems,
      child: child,
    );
  }
}
