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
        _currentItemData.add(
            DataSourceItemRepresentation.simple(formats: [format], data: data));
      }
    });
  }

  void writeLazy<T>(ClipboardType<T> key, FutureOr<T> Function() itemProvider) {
    _actions.add(() {
      final platformKey = key.platformType();
      for (final format in platformKey.writableSystemTypes()) {
        _currentItemData.add(DataSourceItemRepresentation.lazy(
            formats: [format],
            dataProvider: (format) async {
              final value = await itemProvider();
              return await platformKey.convertToSystem(value, format);
            }));
      }
    });
  }

  void nextItem() {
    _actions.add(() {
      _items.add(DataSourceItem(representations: _currentItemData));
      _currentItemData = [];
    });
  }

  Future<Listenable> commitToClipboard() async {
    final data = await _buildWriterData();
    final handle = await data.register();
    await RawClipboardWriter.instance.write(handle);
    return handle.onDispose;
  }

  Future<DataSource> _buildWriterData() async {
    _items = [];
    _currentItemData = [];
    for (final action in _actions) {
      await action();
    }
    if (_currentItemData.isNotEmpty) {
      _items.add(DataSourceItem(representations: _currentItemData));
    }
    return DataSource(_items);
  }

  final _actions = <FutureOr<void> Function()>[];
  List<DataSourceItem> _items = [];
  List<DataSourceItemRepresentation> _currentItemData = [];
}
