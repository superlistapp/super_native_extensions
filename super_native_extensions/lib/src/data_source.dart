import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:meta/meta.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';

/// Low level representation of data that can be written to clipboard or used
/// for drag&drop.
class DataSource {
  DataSource(this.items);

  dynamic serialize() => {
        'items': items.map((e) => e.serialize()),
      };

  final List<DataSourceItem> items;

  /// Registers this source with native code. The source data will be kept alive
  /// until handle is disposed.
  Future<DataSourceHandle> register() {
    return _DataSourceManager.instance.registerDataSource(this);
  }
}

/// Single item of data source. Item can have multiple representation;
class DataSourceItem {
  DataSourceItem(
    this.representations,
  );

  dynamic serialize() => {
        'representations': representations.map((e) => e.serialize()),
      };

  final List<DataSourceItemRepresentation> representations;
}

@sealed
abstract class DataSourceItemRepresentation {
  static DataSourceItemRepresentationSimple simple({
    required List<String> formats,
    required Object data,
  }) =>
      DataSourceItemRepresentationSimple._(
        formats: formats,
        data: data,
      );

  static DataSourceItemRepresentationLazy lazy({
    required List<String> formats,
    required FutureOr<Object> Function(String format) dataProvider,
  }) =>
      DataSourceItemRepresentationLazy._(
        formats: formats,
        dataProvider: dataProvider,
      );

  dynamic serialize();
}

/// Single representation of data source item. Useful when data is known upfront.
class DataSourceItemRepresentationSimple extends DataSourceItemRepresentation {
  DataSourceItemRepresentationSimple._({
    required this.formats,
    required this.data,
  });

  @override
  dynamic serialize() => {
        'type': 'simple',
        'formats': formats,
        'data': data,
      };

  /// List of platform-specific data formats.
  final List<String> formats;
  final Object data;
}

/// Single reprsentation of data source item. Useful when data is generated
/// on demand.
class DataSourceItemRepresentationLazy extends DataSourceItemRepresentation {
  DataSourceItemRepresentationLazy._({
    required this.formats,
    required this.dataProvider,
  }) : id = _nextId++;

  @override
  dynamic serialize() => {
        'type': 'lazy',
        'id': id,
        'formats': formats,
      };

  final int id;
  final List<String> formats;
  final FutureOr<Object> Function(String format) dataProvider;
}

class DataSourceHandle {
  DataSourceHandle._(this.id, this.source);

  final int id;
  final DataSource source;
  Listenable get onDispose => _onDispose;

  final _onDispose = _SimpleNotifier();

  bool _disposed = false;

  Future<void> dispose() async {
    assert(!_disposed);
    _disposed = true;
    _onDispose.notify();
    await _DataSourceManager.instance.unregisterDataSource(id);
  }
}

//
// Internal
//

class _SimpleNotifier extends ChangeNotifier {
  void notify() {
    super.notifyListeners();
  }
}

int _nextId = 1;

class _DataSourceManager {
  _DataSourceManager._() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<DataSourceHandle> registerDataSource(DataSource source) async {
    final id =
        await _channel.invokeMethod("registerDataSource", source.serialize());
    final handle = DataSourceHandle._(id, source);
    _handles[id] = handle;
    for (final item in handle.source.items) {
      for (final data in item.representations) {
        if (data is DataSourceItemRepresentationLazy) {
          _lazyData[data.id] = data;
        }
      }
    }
    return handle;
  }

  Future<void> unregisterDataSource(int sourceId) async {
    await _channel.invokeMethod("unregisterDataSource", sourceId);
    final handle = _handles.remove(sourceId);
    if (handle != null) {
      for (final item in handle.source.items) {
        for (final data in item.representations) {
          if (data is DataSourceItemRepresentationLazy) {
            _lazyData.remove(data.id);
          }
        }
      }
    }
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'getLazyData') {
      final args = call.arguments as Map;
      final id = args["id"] as int;
      final format = args["format"] as String;
      final lazyData = _lazyData[id];
      if (lazyData != null) {
        return _ValuePromiseResult.ok(await lazyData.dataProvider(format))
            .serialize();
      } else {
        return _ValuePromiseResult.cancelled().serialize();
      }
    }
  }

  static final instance = _DataSourceManager._();

  final _channel = NativeMethodChannel('DataSourceManager',
      context: superNativeExtensionsContext);

  final _handles = <int, DataSourceHandle>{};
  final _lazyData = <int, DataSourceItemRepresentationLazy>{};
}

abstract class _ValuePromiseResult {
  static _ValuePromiseResultOk ok(dynamic value) =>
      _ValuePromiseResultOk._(value);

  static _ValuePromiseResultCancelled cancelled() =>
      _ValuePromiseResultCancelled._();

  dynamic serialize();
}

class _ValuePromiseResultCancelled extends _ValuePromiseResult {
  _ValuePromiseResultCancelled._();

  @override
  serialize() => {
        'type': 'cancelled',
      };
}

class _ValuePromiseResultOk extends _ValuePromiseResult {
  _ValuePromiseResultOk._(this.value);

  final dynamic value;

  @override
  serialize() => {
        'type': 'ok',
        'value': value,
      };
}
