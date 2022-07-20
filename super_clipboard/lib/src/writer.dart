import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart';

import 'common.dart';

class ClipboardWriter {
  void write<T>(ClipboardType<T> key, T value) {
    _actions.add(() async {
      final platformKey = key.platformType();
      for (final format in platformKey.writableSystemTypes()) {
        final data = await platformKey.convertToSystem(value, format);
        _currentItemData
            .add(DataRepresentation.simple(format: format, data: data));
      }
    });
  }

  void writeLazy<T>(ClipboardType<T> key, FutureOr<T> Function() itemProvider) {
    _actions.add(() {
      final platformKey = key.platformType();
      for (final format in platformKey.writableSystemTypes()) {
        _currentItemData.add(DataRepresentation.lazy(
            format: format,
            dataProvider: () async {
              final value = await itemProvider();
              return await platformKey.convertToSystem(value, format);
            }));
      }
    });
  }

  void nextItem() {
    _actions.add(() {
      _items.add(DataProvider(representations: _currentItemData));
      _currentItemData = [];
    });
  }

  Future<List<Listenable>> commitToClipboard() async {
    final items = await _buildWriterData();
    final handles = <DataProviderHandle>[];
    for (final item in items) {
      handles.add(await item.register());
    }
    await RawClipboardWriter.instance.write(handles);
    return handles.map((e) => e.onDispose).toList(growable: false);
  }

  Future<List<DataProvider>> _buildWriterData() async {
    _items = [];
    _currentItemData = [];
    for (final action in _actions) {
      await action();
    }
    if (_currentItemData.isNotEmpty) {
      _items.add(DataProvider(representations: _currentItemData));
    }
    return _items;
  }

  final _actions = <FutureOr<void> Function()>[];
  List<DataProvider> _items = [];
  List<DataRepresentation> _currentItemData = [];
}
