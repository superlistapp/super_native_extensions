import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'data_provider.dart';
import 'drop.dart';
import 'image_data.dart';
import 'mutex.dart';
import 'gesture/pointer_device_kind.dart';

import 'native/drag.dart' if (dart.library.js_interop) 'web/drag.dart';
import 'widget_snapshot/widget_snapshot.dart';

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

  DragConfiguration clone() {
    return DragConfiguration(
      items: items.map((e) => e).toList(),
      allowedOperations: allowedOperations,
      animatesToStartingPositionOnCancelOrFail:
          animatesToStartingPositionOnCancelOrFail,
      prefersFullSizePreviews: prefersFullSizePreviews,
    );
  }

  void disposeImages() {
    for (final item in items) {
      item.disposeImages();
    }
  }
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
  TargetedWidgetSnapshot image;

  /// If specified this image will be used for lift animation on iOS and Android.
  TargetedWidgetSnapshot? liftImage;

  final Object? localData;

  void disposeImages() {
    image.dispose();
    liftImage?.dispose();
  }
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
  /// Whether the drag session is in progress. False before drag started
  /// and after drag completed.
  ValueListenable<bool> get dragging;

  /// True when drag session has started.
  bool get dragStarted => dragging.value || dragCompleted.value != null;

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

  @mustCallSuper
  Future<void> initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    PointerDeviceKindDetector.instance;
  }

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
  void cancelSession(DragSession session);

  Future<void> startDrag({
    required BuildContext buildContext,
    required DragSession session,
    required DragConfiguration configuration,
    required ui.Offset position,
    TargetedWidgetSnapshot? combinedDragImage,
  });
}
