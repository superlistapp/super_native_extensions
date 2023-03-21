import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

class DragImage {
  DragImage({
    required this.image,
    this.liftImage,
  });

  /// Image used while dragging
  raw.TargetedImage image;

  /// If specified this image will be used for lift animation on iOS.
  raw.TargetedImage? liftImage;
}

/// Represent single item being dragged in a [DragSession].
class DragItem extends DataWriterItem {
  DragItem({
    super.suggestedName,
    required this.image,
    this.localData,
  });

  @override
  bool get virtualFileSupported =>
      !kIsWeb &&
      (defaultTargetPlatform == TargetPlatform.macOS ||
          defaultTargetPlatform == TargetPlatform.windows ||
          defaultTargetPlatform == TargetPlatform.iOS);

  final FutureOr<DragImage> image;

  /// Data associated with this drag item that can be only read when dropping
  /// within same application. The data must be serializable with
  /// [StandardMessageCodec]. It is possible to read [localData] from
  /// one isolate in another isolate.
  final Object? localData;
}

/// Addtional options for drag session.
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
  final List<DragItem> items;

  /// Allowed drop operation for this session.
  final List<raw.DropOperation> allowedOperations;

  /// Additonal platform sepcific options.
  final DragOptions options;
}
