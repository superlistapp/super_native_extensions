import 'dart:ui' as ui;
import 'dart:ui';

import 'package:collection/collection.dart';
import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import '../api_model.dart';
import '../data_provider.dart';
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
        'image': image.serialize(),
      };
}

extension DragImageExt on DragImage {
  dynamic serialize() => {
        'imageData': imageData.serialize(),
        'sourceRect': sourceRect.serialize(),
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
    final view = await getFlutterView();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod("newContext", {'viewHandle': view});
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'getConfigurationForDragRequest') {
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
    } else if (call.method == 'getAdditionalItemsForLocation') {
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
    } else if (call.method == 'isLocationDraggable') {
      final arguments = call.arguments as Map;
      final location = OffsetExt.deserialize(arguments['location']);
      return delegate?.isLocationDraggable(location) ?? false;
    } else if (call.method == 'releaseDataProvider') {
      final provider = _dataProviders.remove(call.arguments);
      provider?.dispose();
    } else if (call.method == 'dragSessionDidMove') {
      final arguments = call.arguments as Map;
      final sessionId = arguments['sessionId'];
      final screenLocation = OffsetExt.deserialize(arguments['screenLocation']);
      final session = _sessions[sessionId];
      if (session != null) {
        if (!session._started) {
          session._started = true;
          session._dragStarted.notify();
        }
        session._lastScreenLocation.value = screenLocation;
      }
    } else if (call.method == 'dragSessionDidEnd') {
      final arguments = call.arguments as Map;
      final sessionId = arguments['sessionId'];
      final dropOperation =
          DropOperation.values.byName(arguments['dropOperation']);
      final session = _sessions.remove(sessionId);
      session?.dragCompleted.value = dropOperation;
    } else {
      return null;
    }
  }

  Future<DragImage> combineDragImage(DragConfiguration configuration) async {
    var combinedRect = Rect.zero;
    for (final item in configuration.items) {
      if (combinedRect.isEmpty) {
        combinedRect = item.image.sourceRect;
      } else {
        combinedRect = combinedRect.expandToInclude(item.image.sourceRect);
      }
    }
    final scale =
        configuration.items.firstOrNull?.image.imageData.devicePixelRatio ??
            1.0;
    final offset = combinedRect.topLeft;
    final rect = combinedRect.translate(-offset.dx, -offset.dy);
    final recorder = PictureRecorder();
    final canvas = Canvas(recorder);
    canvas.scale(scale, scale);
    for (final item in configuration.items) {
      final image = item.image.imageData.sourceImage;
      final destinationRect =
          item.image.sourceRect.translate(-offset.dx, -offset.dy);
      canvas.drawImageRect(
          image,
          Rect.fromLTWH(0, 0, image.width.toDouble(), image.height.toDouble()),
          destinationRect,
          Paint());
    }
    final picture = recorder.endRecording();
    final image = await picture.toImage(
        (rect.width * scale).ceil(), (rect.height * scale).ceil());

    return DragImage(
      imageData: await ImageData.fromImage(image, devicePixelRatio: scale),
      sourceRect: combinedRect,
    );
  }

  @override
  DragSession newSession() {
    return DragSessionImpl(dragContext: this);
  }

  Future<List<Object?>?> getLocalData(int sessionId) async {
    return _channel.invokeMethod('getLocalData', {
      'sessionId': sessionId,
    });
  }

  @override
  Future<DragSession> startDrag({
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
    return session;
  }
}
