import 'dart:async';

import 'package:super_data_transfer/super_data_transfer.dart';

import 'common.dart';

class ClipboardWriter {
  void write<T>(ClipboardKey<T> key, T value) {
    _actions.add(() async {
      final platformKey = key.platformKey();
      for (final type in platformKey.writableSystemTypes()) {
        final data = await platformKey.convertToSystem(value, type);
        _currentItemData
            .add(RawClipboardWriterItemData.simple(types: [type], data: data));
      }
    });
  }

  void writeLazy<T>(ClipboardKey<T> key, FutureOr<T> Function() itemProvider) {
    _actions.add(() {
      final platformKey = key.platformKey();
      for (final type in platformKey.writableSystemTypes()) {
        _currentItemData.add(RawClipboardWriterItemData.lazy(
            types: [type],
            dataProvider: () async {
              final value = await itemProvider();
              return await platformKey.convertToSystem(value, type);
            }));
      }
    });
  }

  void nextItem() {
    _actions.add(() {
      _items.add(RawClipboardWriterItem(_currentItemData));
      _currentItemData = [];
    });
  }

  static RawClipboardWriter? _currentWriter;

  Future<void> commitToClipboard() async {
    final data = await _buildWriterData();
    final writer = await RawClipboardWriter.withData(data);
    final previousWriter = _currentWriter;
    _currentWriter = writer;
    await writer.writeToClipboard();
    if (previousWriter != null) {
      await previousWriter.dispose();
    }
  }

  Future<RawClipboardWriterData> _buildWriterData() async {
    _items = [];
    _currentItemData = [];
    for (final action in _actions) {
      await action();
    }
    if (_currentItemData.isNotEmpty) {
      _items.add(RawClipboardWriterItem(_currentItemData));
    }
    return RawClipboardWriterData(_items);
  }

  final _actions = <FutureOr<void> Function()>[];
  List<RawClipboardWriterItem> _items = [];
  List<RawClipboardWriterItemData> _currentItemData = [];
}
