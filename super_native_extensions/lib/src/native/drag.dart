import 'dart:ui' as ui;

import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import '../api_model.dart';
import '../data_provider.dart';
import '../drag_internal.dart';
import '../drag.dart';
import '../util.dart';
import 'api_model.dart';
import 'context.dart';

extension DragConfigurationExt on DragConfiguration {
  dynamic serialize() => {
        'items': items.map((e) => e.serialize()),
        'allowedOperations': allowedOperations.map((e) => e.name),
        'animatesToStartingPositionOnCancelOrFail':
            animatesToStartingPositionOnCancelOrFail,
        'prefersFullSizePreviews': prefersFullSizePreviews,
      };
}

extension DragItemExt on DragItem {
  dynamic serialize() => {
        'dataProviderId': dataProvider.id,
        'localData': localData,
        'image': image.image.serialize(),
        'liftImage': image.liftImage?.serialize(),
      };
}

extension DragRequestExt on DragRequest {
  dynamic serialize() => {
        'configuration': configuration.serialize(),
        'position': position.serialize(),
        'combinedDragImage': combinedDragImage?.serialize(),
      };
}

class DragSessionImpl extends DragSession {
  DragSessionImpl({
    required this.dragContext,
  });

  @override
  bool get dragging => _started;

  final DragContextImpl dragContext;

  @override
  Listenable get dragStarted => _dragStarted;
  @override
  ValueNotifier<DropOperation?> get dragCompleted => _dragCompleted;
  @override
  ValueNotifier<ui.Offset?> get lastScreenLocation => _lastScreenLocation;

  Listenable get sessionIsDoneWithDataSource => _sessionIsDoneWithDataSource;

  int? sessionId;

  @override
  Future<List<Object?>?> getLocalData() async {
    if (sessionId != null) {
      return dragContext.getLocalData(sessionId!);
    } else {
      return [];
    }
  }

  final _dragStarted = SimpleNotifier();
  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _lastScreenLocation = ValueNotifier<ui.Offset?>(null);
  final _sessionIsDoneWithDataSource = SimpleNotifier();

  bool _started = false;
}

final _channel =
    NativeMethodChannel('DragManager', context: superNativeExtensionsContext);

class DragContextImpl extends DragContext {
  DragContextImpl();

  final _sessions = <int, DragSessionImpl>{};
  final _dataProviders = <int, DataProviderHandle>{};

  @override
  Future<void> initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
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
          return {'configuration': await configuration.serialize()};
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
        }
        return {'items': items?.map((e) => e.serialize())};
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
          if (!session._started) {
            session._started = true;
            session._dragStarted.notify();
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
        session?.dragCompleted.value = dropOperation;
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

  Future<List<Object?>?> getLocalData(int sessionId) async {
    return _channel.invokeMethod('getLocalData', {
      'sessionId': sessionId,
    });
  }

  @override
  Future<void> startDrag({
    required DragSession session,
    required DragConfiguration configuration,
    required Offset position,
  }) async {
    final needsCombinedDragImage =
        (await _channel.invokeMethod('needsCombinedDragImage')) as bool;
    final request = DragRequest(
      configuration: configuration,
      position: position,
      combinedDragImage:
          needsCombinedDragImage ? await combineDragImage(configuration) : null,
    );
    final sessionId =
        await _channel.invokeMethod("startDrag", request.serialize());
    final sessionImpl = session as DragSessionImpl;
    sessionImpl.sessionId = sessionId;
    _sessions[sessionId] = sessionImpl;
    for (final item in request.configuration.items) {
      _dataProviders[item.dataProvider.id] = item.dataProvider;
    }
  }
}
