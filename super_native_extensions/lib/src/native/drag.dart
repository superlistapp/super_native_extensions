import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import '../data_provider.dart';
import '../drag_internal.dart';
import '../drag.dart';
import '../drop.dart';
import '../image_data.dart';
import '../util.dart';
import '../widget_snapshot/widget_snapshot.dart';
import 'image_data.dart';
import 'context.dart';

extension DragConfigurationExt on DragConfiguration {
  Future<dynamic> serialize() async => {
        'items': await Future.wait(items.map((e) => e.serialize())),
        'allowedOperations': allowedOperations.map((e) => e.name),
        'animatesToStartingPositionOnCancelOrFail':
            animatesToStartingPositionOnCancelOrFail,
        'prefersFullSizePreviews': prefersFullSizePreviews,
      };
}

extension DragItemExt on DragItem {
  Future<dynamic> serialize() async => {
        'dataProviderId': dataProvider.id,
        'localData': localData,
        'image': (await image.intoRaw()).serialize(),
        'liftImage': (await liftImage?.intoRaw())?.serialize()
      };
}

extension DragRequestExt on DragRequest {
  Future<dynamic> serialize() async => {
        'configuration': await configuration.serialize(),
        'position': position.serialize(),
        'combinedDragImage': combinedDragImage?.serialize(),
      };
}

class DragSessionImpl extends DragSession {
  DragSessionImpl({
    required this.dragContext,
  });

  final DragContextImpl dragContext;

  @override
  ValueListenable<bool> get dragging => _dragging;

  @override
  ValueListenable<DropOperation?> get dragCompleted => _dragCompleted;

  @override
  ValueListenable<ui.Offset?> get lastScreenLocation => _lastScreenLocation;

  int? sessionId;

  @override
  Future<List<Object?>?> getLocalData() async {
    if (sessionId != null) {
      return dragContext.getLocalData(sessionId!);
    } else {
      return [];
    }
  }

  void dispose() {
    _dragging.dispose();
    _dragCompleted.dispose();
    _lastScreenLocation.dispose();
  }

  final _dragging = ValueNotifier<bool>(false);
  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _lastScreenLocation = ValueNotifier<ui.Offset?>(null);
}

final _channel =
    NativeMethodChannel('DragManager', context: superNativeExtensionsContext);

class DragContextImpl extends DragContext {
  DragContextImpl();

  final _sessions = <int, DragSessionImpl>{};
  final _dataProviders = <int, DataProviderHandle>{};

  @override
  Future<void> initialize() async {
    super.initialize();
    final engineHandle = await EngineContext.instance.getEngineHandle();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod('newContext', {
      'engineHandle': engineHandle,
    });
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'getConfigurationForDragRequest') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final location = OffsetExt.deserialize(arguments['location']);
        final sessionId = arguments['sessionId'];
        final session = DragSessionImpl(dragContext: this);
        final configuration = await delegate?.getConfigurationForDragRequest(
          location: location,
          session: session,
        );
        if (configuration != null) {
          session.sessionId = sessionId;
          _sessions[sessionId] = session;
          for (final item in configuration.items) {
            _dataProviders[item.dataProvider.id] = item.dataProvider;
          }
          final res = {'configuration': await configuration.serialize()};
          configuration.disposeImages();
          return res;
        } else {
          return {'configuration': null};
        }
      }, () => {'configuration': null});
    } else if (call.method == 'getAdditionalItemsForLocation') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final location = OffsetExt.deserialize(arguments['location']);
        final sessionId = arguments['sessionId'];
        final session = _sessions[sessionId];
        List<DragItem>? items;
        if (session != null) {
          items = await delegate?.getAdditionalItemsForLocation(
            location: location,
            session: session,
          );
        }
        if (items != null) {
          for (final item in items) {
            _dataProviders[item.dataProvider.id] = item.dataProvider;
          }
          final res = {
            'items': await Future.wait(items.map((e) => e.serialize())),
          };
          for (final item in items) {
            item.disposeImages();
          }
          return res;
        } else {
          return null;
        }
      }, () => {'items': null});
    } else if (call.method == 'isLocationDraggable') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final location = OffsetExt.deserialize(arguments['location']);
        return delegate?.isLocationDraggable(location) ?? false;
      }, () => false);
    } else if (call.method == 'releaseDataProvider') {
      return handleError(() async {
        final provider = _dataProviders.remove(call.arguments);
        provider?.dispose();
      }, () => null);
    } else if (call.method == 'dragSessionDidMove') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final sessionId = arguments['sessionId'];
        final screenLocation =
            OffsetExt.deserialize(arguments['screenLocation']);
        final session = _sessions[sessionId];
        if (session != null) {
          if (!session._dragging.value &&
              session._dragCompleted.value == null) {
            session._dragging.value = true;
          }
          session._lastScreenLocation.value = screenLocation;
        }
      }, () => null);
    } else if (call.method == 'dragSessionDidEnd') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final sessionId = arguments['sessionId'];
        final dropOperation =
            DropOperation.values.byName(arguments['dropOperation']);
        final session = _sessions.remove(sessionId);
        if (session != null) {
          session._dragging.value = false;
          session._dragCompleted.value = dropOperation;
          session.dispose();
        }
      }, () => null);
    } else {
      return null;
    }
  }

  @override
  DragSession newSession({
    int? pointer,
  }) {
    return DragSessionImpl(dragContext: this);
  }

  @override
  void cancelSession(DragSession session) {
    final sessionImpl = session as DragSessionImpl;
    assert(sessionImpl.dragCompleted.value == null);
    assert(sessionImpl.dragging.value == false);
    sessionImpl._dragCompleted.value = DropOperation.userCancelled;
    session.dispose();
  }

  Future<List<Object?>?> getLocalData(int sessionId) async {
    return _channel.invokeMethod('getLocalData', {
      'sessionId': sessionId,
    });
  }

  @override
  Future<void> startDrag({
    required BuildContext buildContext,
    required DragSession session,
    required DragConfiguration configuration,
    required Offset position,
    TargetedWidgetSnapshot? combinedDragImage,
  }) async {
    final needsCombinedDragImage =
        (await _channel.invokeMethod('needsCombinedDragImage')) as bool;
    final request = DragRequest(
      configuration: configuration,
      position: position,
      combinedDragImage: needsCombinedDragImage
          ? (await combinedDragImage?.intoRaw()) ??
              await combineDragImage(configuration)
          : null,
    );

    final sessionId =
        await _channel.invokeMethod("startDrag", await request.serialize());
    final sessionImpl = session as DragSessionImpl;
    sessionImpl.sessionId = sessionId;
    _sessions[sessionId] = sessionImpl;
    for (final item in request.configuration.items) {
      _dataProviders[item.dataProvider.id] = item.dataProvider;
    }
  }
}
