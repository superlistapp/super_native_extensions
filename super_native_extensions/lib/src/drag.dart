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
  Listenable get sessionIsDoneWithDataSource => _sessionIsDoneWithDataSource;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _sessionIsDoneWithDataSource = SimpleNotifier();
}

abstract class RawDragContextDelegate {
  Future<DataSourceHandle?> getDataSourceForDragRequest({
    required ui.Offset location,
    required DragSession
        session, // session will be unused if null handle is returned
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
    if (call.method == 'dataSourceForDragRequest') {
      final arguments = call.arguments as Map;
      final location = OffsetExt.deserialize(arguments['location']);
      final sessionId = arguments['sessionId'];
      final session = DragSession();
      final source = await _delegate?.getDataSourceForDragRequest(
        location: location,
        session: session,
      );
      if (source != null) {
        source.onDispose.addListener(() {
          session._sessionIsDoneWithDataSource.notify();
        });
        _sessions[sessionId] = session;
        _dataSources[source.id] = source;
        return {'dataSourceId': source.id};
      } else {
        return null;
      }
    } else if (call.method == 'releaseDataSource') {
      final source = _dataSources.remove(call.arguments);
      source?.dispose();
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

  /// Returns drag session Id
  Future<DragSession> startDrag({
    required DragRequest request,
  }) async {
    final sessionId =
        await _channel.invokeMethod("startDrag", await request.serialize());
    final session = DragSession();
    request.dataSource.onDispose.addListener(() {
      session._sessionIsDoneWithDataSource.notify();
    });
    _sessions[sessionId] = session;
    return session;
  }
}

class DragRequest {
  DragRequest({
    required this.dataSource,
    required this.pointInRect,
    required this.dragPosition,
    required this.devicePixelRatio,
    required this.image,
  });

  final DataSourceHandle dataSource;
  final Offset pointInRect;
  final Offset dragPosition;
  final double devicePixelRatio;
  final ui.Image image;

  Future<dynamic> serialize() async {
    final imageData =
        await ImageData.fromImage(image, devicePixelRatio: devicePixelRatio);
    return {
      'dataSourceId': dataSource.id,
      'pointInRect': pointInRect.serialize(),
      'dragPosition': dragPosition.serialize(),
      'image': imageData.serialize(),
    };
  }
}
