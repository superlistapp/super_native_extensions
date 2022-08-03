import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';

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
  ValueListenable<DropOperation?> get dragCompleted;
  ValueListenable<ui.Offset?> get lastScreenLocation;
}

abstract class DragContextDelegate {
  Future<DragConfiguration?> getConfigurationForDragRequest({
    required ui.Offset location,
    // session will be unused if null handle is returned
    required DragSession session,
  });

  bool isLocationDraggable(ui.Offset location);
}

abstract class DragContext {
  set delegate(DragContextDelegate? delegate) {
    _delegate = delegate;
  }

  static final _mutex = Mutex();

  static DragContext? _instance;

  @protected
  DragContextDelegate? get delegate => _delegate;

  DragContextDelegate? _delegate;

  @protected
  Future<void> initialize();

  static Future<DragContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = DragContextImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }

  DragSession newSession();

  Future<void> startDrag({
    required DragSession session,
    required DragConfiguration configuration,
    required ui.Offset position,
  });
}
