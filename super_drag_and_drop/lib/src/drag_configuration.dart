import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:super_clipboard/super_clipboard.dart';

import 'model.dart';

/// Represent single item being dragged in a [DragSession].
class DragItem extends DataWriterItem {
  DragItem({
    super.suggestedName,
    this.localData,
  });

  @override
  bool get virtualFileSupported =>
      !kIsWeb &&
      (defaultTargetPlatform == TargetPlatform.macOS ||
          defaultTargetPlatform == TargetPlatform.windows ||
          defaultTargetPlatform == TargetPlatform.iOS);

  /// Data associated with this drag item that can be only read when dropping
  /// within same application. The data must be serializable with
  /// [StandardMessageCodec]. It is possible to read [localData] from
  /// one isolate in another isolate.
  final Object? localData;
}

/// Single item of [DragConfiguration] consisting of drag item and corresponding
/// image.
class DragConfigurationItem {
  DragConfigurationItem({
    required this.item,
    required this.image,
    this.liftImage,
  });

  final DragItem item;
  final TargetedWidgetSnapshot image;
  final TargetedWidgetSnapshot? liftImage;
}

/// Additional options for drag session.
class DragOptions {
  const DragOptions({
    this.animatesToStartingPositionOnCancelOrFail = true,
    this.prefersFullSizePreviews = true,
  });

  /// macOS specific
  final bool animatesToStartingPositionOnCancelOrFail;

  /// iOS specific
  final bool prefersFullSizePreviews;
}

/// Initial configuration of a drag session.
class DragConfiguration {
  DragConfiguration({
    required this.items,
    required this.allowedOperations,
    this.options = const DragOptions(),
  });

  /// List of items in this session.
  final List<DragConfigurationItem> items;

  /// Allowed drop operation for this session.
  final List<DropOperation> allowedOperations;

  /// Additional platform specific options.
  final DragOptions options;
}
