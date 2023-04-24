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

class DragImageData {
  DragImageData({
    required this.image,
    required this.imageSource,
    this.liftImage,
    this.liftImageSource,
  });

  /// Image used while dragging.
  TargetedImageData image;
  TargetedImage imageSource;

  /// Used on iPad during lift (before dragging starts). If not set normal
  /// drag image is used. This should closely resemble the widget being dragged.
  TargetedImageData? liftImage;
  TargetedImage? liftImageSource;
}

class DragItem {
  DragItem({
    required this.dataProvider,
    required this.image,
    required this.liftImage,
    this.localData,
  });

  final DataProviderHandle dataProvider;

  /// Image used while dragging
  TargetedImage image;

  /// If specified this image will be used for lift animation on iOS and Android.
  TargetedImage? liftImage;

  final Object? localData;
}

class DragRequest {
  DragRequest({
    required this.configuration,
    required this.position,
    this.combinedDragImage,
  });

  final DragConfiguration configuration;
  final ui.Offset position;
  final TargetedImageData? combinedDragImage;
}

/// Represents a drag session. Allows inspecting local drag data and
/// provides notifications about drag state changes.
abstract class DragSession {
  /// Fired when session dragging started.
  Listenable get dragStarted;

  /// True when session is already in progress.
  bool get dragging;

  /// Fired on drag completion. The value will contain drop operation that the
  /// drag finished with.
  ValueListenable<DropOperation?> get dragCompleted;

  /// Updated when drag session moves. On mobile and web you will only
  /// get notified when moving over application Window.
  /// On desktop platforms the notification covers entire screen.
  ValueListenable<ui.Offset?> get lastScreenLocation;

  /// Returns local data for each of the draggable items in current session.
  /// Will return `null` if drag session not local, not yet active or already
  /// completed.
  Future<List<Object?>?> getLocalData();
}

abstract class DragContextDelegate {
  Future<DragConfiguration?>? getConfigurationForDragRequest({
    required ui.Offset location,
    // session will be unused if null handle is returned
    required DragSession session,
  });

  Future<List<DragItem>?> getAdditionalItemsForLocation({
    required ui.Offset location,
    required DragSession session,
  });

  bool isLocationDraggable(ui.Offset location);
}

abstract class DragContext {
  static final _mutex = Mutex();

  static DragContext? _instance;

  DragContextDelegate? delegate;

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

  DragSession newSession({int? pointer});

  Future<void> startDrag({
    required DragSession session,
    required DragConfiguration configuration,
    required ui.Offset position,
    TargetedImageData? combinedDragImage,
  });
}
