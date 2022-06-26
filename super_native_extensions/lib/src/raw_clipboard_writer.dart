import 'dart:async';
import 'package:flutter/services.dart';
import 'package:meta/meta.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

import 'context.dart';

//
// Low level interface for setting clipboard items
//

class RawClipboardWriter {
  static Future<RawClipboardWriter> withData(RawClipboardWriterData data) {
    return _RawClipboardWriterManager.instance.createWriter(data);
  }

  Future<void> writeToClipboard() async {
    await _RawClipboardWriterManager.instance.writeToClipboard(_handle);
  }

  Future<void> dispose() async {
    _RawClipboardWriterManager.instance.disposeWriter(_handle);
  }

  int get handle => _handle;

  RawClipboardWriter._(this._handle, this.data);

  final RawClipboardWriterData data;
  final int _handle;
}

class RawClipboardWriterData {
  RawClipboardWriterData(this.items);

  dynamic serialize() => {
        'items': items.map((e) => e.serialize()),
      };

  final List<RawClipboardWriterItem> items;
}

class RawClipboardWriterItem {
  RawClipboardWriterItem(
    this.data,
  );

  dynamic serialize() => {
        'data': data.map((e) => e.serialize()),
      };

  final List<RawClipboardWriterItemData> data;
}

@sealed
abstract class RawClipboardWriterItemData {
  static RawClipboardWriterItemDataSimple simple({
    required List<String> types,
    required Object data,
  }) =>
      RawClipboardWriterItemDataSimple._(
        types: types,
        data: data,
      );

  static RawClipboardWriterItemDataLazy lazy({
    required List<String> types,
    required FutureOr<Object> Function() dataProvider,
  }) =>
      RawClipboardWriterItemDataLazy._(
        types: types,
        dataProvider: dataProvider,
      );

  static RawClipboardWriterItemDataVirtualFile virtualFile({
    required FutureOr<Stream<List<int>>> Function() streamProvider,
    required String fileName,
    required int fileSize,
  }) =>
      RawClipboardWriterItemDataVirtualFile._(
        streamProvider: streamProvider,
        fileName: fileName,
        fileSize: fileSize,
      );

  dynamic serialize();
}

class RawClipboardWriterItemDataSimple extends RawClipboardWriterItemData {
  RawClipboardWriterItemDataSimple._({
    required this.types,
    required this.data,
  });

  @override
  dynamic serialize() => {
        'type': 'simple',
        'types': types,
        'data': data,
      };

  final List<String> types;
  final Object data;
}

class RawClipboardWriterItemDataLazy extends RawClipboardWriterItemData {
  RawClipboardWriterItemDataLazy._({
    required this.types,
    required this.dataProvider,
  }) : id = _nextId++;

  @override
  dynamic serialize() => {
        'type': 'lazy',
        'id': id,
        'types': types,
      };

  final int id;
  final List<String> types;
  final FutureOr<Object> Function() dataProvider;
}

class RawClipboardWriterItemDataVirtualFile extends RawClipboardWriterItemData {
  RawClipboardWriterItemDataVirtualFile._({
    required this.streamProvider,
    required this.fileName,
    required this.fileSize,
  });

  @override
  dynamic serialize() => {
        'type': 'virtualFile',
        'fileSize': fileSize,
        'fileName': fileName,
      };

  final FutureOr<Stream<List>> Function() streamProvider;
  final String fileName;
  final int fileSize;
}

//
// Internal
//

int _nextId = 1;

class _RawClipboardWriterManager {
  _RawClipboardWriterManager._() {
    _channel.setMethodCallHandler(_onMethodCall);
  }

  Future<RawClipboardWriter> createWriter(RawClipboardWriterData data) async {
    final id = await _channel.invokeMethod(
        "registerClipboardWriter", data.serialize());
    final writer = RawClipboardWriter._(id, data);
    _writers[id] = writer;
    for (final item in writer.data.items) {
      for (final data in item.data) {
        if (data is RawClipboardWriterItemDataLazy) {
          _lazyData[data.id] = data;
        }
      }
    }
    return writer;
  }

  Future<void> writeToClipboard(int handleId) async {
    await _channel.invokeMethod('writeToClipboard', handleId);
  }

  Future<void> disposeWriter(int writerId) async {
    await _channel.invokeMethod("unregisterClipboardWriter", writerId);
    final writer = _writers.remove(writerId);
    if (writer != null) {
      for (final item in writer.data.items) {
        for (final data in item.data) {
          if (data is RawClipboardWriterItemDataLazy) {
            _lazyData.remove(data.id);
          }
        }
      }
    }
  }

  Future<dynamic> _onMethodCall(MethodCall call) async {
    if (call.method == 'getLazyData') {
      final id = call.arguments as int;
      final lazyData = _lazyData[id];
      if (lazyData != null) {
        return _ValuePromiseResult.ok(await lazyData.dataProvider())
            .serialize();
      } else {
        return _ValuePromiseResult.cancelled().serialize();
      }
    }
  }

  static final instance = _RawClipboardWriterManager._();

  final _channel = NativeMethodChannel('ClipboardWriterManager',
      context: superNativeExtensionsContext);

  final _writers = <int, RawClipboardWriter>{};
  final _lazyData = <int, RawClipboardWriterItemDataLazy>{};
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
