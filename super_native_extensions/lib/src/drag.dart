import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';
import 'data_provider.dart';
import 'mutex.dart';
import 'api_model.dart';
import 'util.dart';

class DragSession {
  ValueNotifier<DropOperation?> get dragCompleted => _dragCompleted;
  ValueNotifier<ui.Offset?> get lastScreenLocation => _lastScreenLocation;
  Listenable get sessionIsDoneWithDataSource => _sessionIsDoneWithDataSource;

  final _dragCompleted = ValueNotifier<DropOperation?>(null);
  final _lastScreenLocation = ValueNotifier<ui.Offset?>(null);
  final _sessionIsDoneWithDataSource = SimpleNotifier();
}

abstract class RawDragContextDelegate {
  Future<DragConfiguration?> getConfigurationForDragRequest({
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
  final _dataProviders = <int, DataProviderHandle>{};

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
    if (call.method == 'getConfigurationForDragRequest') {
      final arguments = call.arguments as Map;
      final location = OffsetExt.deserialize(arguments['location']);
      final sessionId = arguments['sessionId'];
      final session = DragSession();
      final configuration = await _delegate?.getConfigurationForDragRequest(
        location: location,
        session: session,
      );
      if (configuration != null) {
        // TODO!
        // configuration.dataSource.onDispose.addListener(() {
        //   session._sessionIsDoneWithDataSource.notify();
        // });
        _sessions[sessionId] = session;
        for (final item in configuration.items) {
          _dataProviders[item.dataProvider.id] = item.dataProvider;
        }
        return {'configuration': await configuration.serialize()};
      } else {
        return null;
      }
    } else if (call.method == 'releaseDataProvider') {
      final provider = _dataProviders.remove(call.arguments);
      provider?.dispose();
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
    final sessionId =
        await _channel.invokeMethod("startDrag", request.serialize());
    final session = DragSession();
    // TODO
    // dataSource.onDispose.addListener(() {
    //   session._sessionIsDoneWithDataSource.notify();
    // });
    _sessions[sessionId] = session;
    for (final item in request.configuration.items) {
      _dataProviders[item.dataProvider.id] = item.dataProvider;
    }
    return session;
  }
}


