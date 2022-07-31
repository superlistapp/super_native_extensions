import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';

import 'api_model.dart';
import 'data_provider.dart';
import 'mutex.dart';

import 'native/drag.dart' if (dart.library.js) 'web/drag.dart';

class DragConfiguration {
  DragConfiguration({
    required this.items,
    required this.allowedOperations,
    this.animatesToStartingPositionOnCancelOrFail = true,
    this.prefersFullSizePreviews = false,
  });

  final List<DragItem> items;
  final List<DropOperation> allowedOperations;

  /// macOS specific
  final bool animatesToStartingPositionOnCancelOrFail;

  /// iOS specific
  final bool prefersFullSizePreviews;
}

class DragItem {
  DragItem({
    required this.dataProvider,
    this.liftImage,
    required this.image,
    this.localData,
  });

  final DataProviderHandle dataProvider;

  /// Used on iPad during lift (before dragging starts). If not set normal
  /// drag image is used. This should closely resemble the widget being dragged.
  final DragImage? liftImage;

  /// Image used while dragging.
  final DragImage image;
  final Object? localData;
}

class DragImage {
  DragImage({
    required this.imageData,
    required this.sourceRect,
  });

  final ImageData imageData;
  final ui.Rect sourceRect;
}

class DragRequest {
  DragRequest({
    required this.configuration,
    required this.position,
    this.combinedDragImage,
  });

  final DragConfiguration configuration;
  final ui.Offset position;
  final DragImage? combinedDragImage;
}

abstract class DragSession {
  Listenable get dragStarted;
  ValueNotifier<DropOperation?> get dragCompleted;
  ValueNotifier<ui.Offset?> get lastScreenLocation;
}

abstract class RawDragContextDelegate {
  Future<DragConfiguration?> getConfigurationForDragRequest({
    required ui.Offset location,
    // session will be unused if null handle is returned
    required DragSession session,
  });

  bool isLocationDraggable(ui.Offset location);
}

abstract class RawDragContext {
  set delegate(RawDragContextDelegate? delegate) {
    _delegate = delegate;
  }

  static final _mutex = Mutex();

  static RawDragContext? _instance;

  @protected
  RawDragContextDelegate? get delegate => _delegate;

  RawDragContextDelegate? _delegate;

  @protected
  Future<void> initialize();

  static Future<RawDragContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDragContextImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }

  Future<DragSession> startDrag({
    required DragConfiguration configuration,
    required ui.Offset position,
  });
}
