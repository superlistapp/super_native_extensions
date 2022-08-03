import 'dart:ui' as ui;

import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

class DragImage {
  DragImage(this.image, this.sourceRect);

  final ui.Image image;
  final ui.Rect sourceRect;
}

class DragItem extends ClipboardWriterItem {
  DragItem({
    this.suggestedName,
    this.liftImage,
    required this.image,
    this.localData,
  });

  final String? suggestedName;
  final DragImage? liftImage;
  final DragImage image;
  final Object? localData;
}

class DragOptions {
  const DragOptions({
    this.animatesToStartingPositionOnCancelOrFail = true,
    this.prefersFullSizePreviews = false,
  });

  /// macOS specific
  final bool animatesToStartingPositionOnCancelOrFail;

  /// iOS specific
  final bool prefersFullSizePreviews;
}

class DragConfiguration {
  DragConfiguration({
    required this.items,
    required this.allowedOperations,
    this.options = const DragOptions(),
  });

  final List<DragItem> items;
  final List<raw.DropOperation> allowedOperations;
  final DragOptions options;
}
