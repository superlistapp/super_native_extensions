import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';

import 'api_model.dart';
import 'mutex.dart';

import 'native/drag.dart' if (dart.library.js) 'web/drag.dart';

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
