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
        await _channel.invokeMethod("startDrag", await request.serialize());
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

class DragItem {
  DragItem({
    required this.dataProvider,
    required this.dragImage,
    this.localData,
  });

  Future<dynamic> serialize() async => {
        'dataProviderId': dataProvider.id,
        'localData': localData,
        'image': await dragImage.serialize(),
      };

  final DataProviderHandle dataProvider;
  final DragImage dragImage;
  final Object? localData;
}

class DragConfiguration {
  DragConfiguration({
    required this.items,
    required this.allowedOperations,
    this.animatesToStartingPositionOnCancelOrFail = true,
  });

  final List<DragItem> items;

  final List<DropOperation> allowedOperations;

  /// macOS specific
  final bool animatesToStartingPositionOnCancelOrFail;

  Future<dynamic> _serializeItems() async {
    final res = <dynamic>[];
    for (final item in items) {
      res.add(await item.serialize());
    }
    return res;
  }

  Future<dynamic> serialize() async => {
        'items': await _serializeItems(),
        'allowedOperations': allowedOperations.map((e) => e.name),
        'animatesToStartingPositionOnCancelOrFail':
            animatesToStartingPositionOnCancelOrFail,
      };
}

class DragRequest {
  DragRequest({
    required this.configuration,
    required this.position,
  });

  final DragConfiguration configuration;
  final Offset position;

  Future<dynamic> serialize() async => {
        'configuration': await configuration.serialize(),
        'position': position.serialize(),
      };
}
