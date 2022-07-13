import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/raw_clipboard.dart';
import 'package:super_native_extensions/src/context.dart';
import 'package:super_native_extensions/src/drag_common.dart';
import 'package:super_native_extensions/src/util.dart';

import 'mutex.dart';
import 'api_model.dart';

class DragSession {
  ValueNotifier<DropOperation?> get dragCompleted => _dragCompleted;
  ValueNotifier<ui.Offset?> get lastScreenLocation => _lastScreenLocation;
  Listenable get sessionIsDoneWithDataSource => _sessionIsDoneWithDataSource;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _lastScreenLocation = ValueNotifier<ui.Offset?>(null);
  final _sessionIsDoneWithDataSource = SimpleNotifier();
}

abstract class RawDragContextDelegate {
  Future<DragData?> getDataForDragRequest({
    required ui.Offset location,
    // session will be unused if null handle is returned
    required DragSession session,
  });
}

final _channel =
    NativeMethodChannel('DragManager', context: superNativeExtensionsContext);

class RawDragContext {
  RawDragContext._();

  static RawDragContext? _instance;
  static final _mutex = Mutex();

  set delegate(RawDragContextDelegate? delegate) {
    _delegate = delegate;
  }

  RawDragContextDelegate? _delegate;

  final _sessions = <int, DragSession>{};
  final _dataSources = <int, DataSourceHandle>{};

  Future<void> _initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    final view = await getFlutterView();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod("newContext", {'viewHandle': view});
  }

  static Future<RawDragContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDragContext._();
        await _instance!._initialize();
      }
      return _instance!;
    });
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'getDataForDragRequest') {
      final arguments = call.arguments as Map;
      final location = OffsetExt.deserialize(arguments['location']);
      final sessionId = arguments['sessionId'];
      final session = DragSession();
      final dragData = await _delegate?.getDataForDragRequest(
        location: location,
        session: session,
      );
      if (dragData != null) {
        dragData.dataSource.onDispose.addListener(() {
          session._sessionIsDoneWithDataSource.notify();
        });
        _sessions[sessionId] = session;
        _dataSources[dragData.dataSource.id] = dragData.dataSource;
        return {'dragData': await dragData.serialize()};
      } else {
        return null;
      }
    } else if (call.method == 'releaseDataSource') {
      final source = _dataSources.remove(call.arguments);
      source?.dispose();
    } else if (call.method == 'dragSessionDidMove') {
      final arguments = call.arguments as Map;
      final sessionId = arguments['sessionId'];
      final screenLocation = OffsetExt.deserialize(arguments['screenLocation']);
      final session = _sessions[sessionId];
      if (session != null) {
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

  Future<DragSession> startDrag({
    required DragRequest request,
  }) async {
    final dataSource = request.dragData.dataSource;
    final sessionId =
        await _channel.invokeMethod("startDrag", await request.serialize());
    final session = DragSession();
    dataSource.onDispose.addListener(() {
      session._sessionIsDoneWithDataSource.notify();
    });
    _sessions[sessionId] = session;
    _dataSources[dataSource.id] = dataSource;
    return session;
  }
}

class DragImage {
  DragImage({
    required this.image,
    required this.pointInRect,
    required this.devicePixelRatio,
  });

  Future<dynamic> serialize() async {
    final imageData =
        await ImageData.fromImage(image, devicePixelRatio: devicePixelRatio);
    return {
      'imageData': imageData.serialize(),
      'pointInRect': pointInRect.serialize(),
    };
  }

  final ui.Image image;
  final Offset pointInRect;
  final double devicePixelRatio;
}

class DragData {
  DragData({
    required this.allowedOperations,
    required this.dataSource,
    required this.dragImage,
  });

  final List<DropOperation> allowedOperations;
  final DataSourceHandle dataSource;
  final DragImage dragImage;

  Future<dynamic> serialize() async => {
        'allowedOperations': allowedOperations.map((e) => e.name),
        'dataSourceId': dataSource.id,
        'dragImage': await dragImage.serialize(),
      };
}

class DragRequest {
  DragRequest({
    required this.dragData,
    required this.dragPosition,
  });

  final DragData dragData;
  final Offset dragPosition;

  Future<dynamic> serialize() async => {
        'dragData': await dragData.serialize(),
        'dragPosition': dragPosition.serialize(),
      };
}
