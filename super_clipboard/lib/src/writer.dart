import 'dart:async';

import 'package:flutter/material.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'encoded_data.dart';
import 'util.dart';

class ClipboardWriterItem {
  void addData(FutureOr<EncodedData> data) {
    _data.add(data);
  }

  Listenable get onRegistered => _onRegistered;
  Listenable get onDisposed => _onDisposed;

  final _onRegistered = SimpleNotifier();
  final _onDisposed = SimpleNotifier();
  final _data = <FutureOr<EncodedData>>[];
}

class ClipboardWriter {
  ClipboardWriter._();

  Future<void> write(Iterable<ClipboardWriterItem> items) async {
    final providers = <Pair<raw.DataProvider, ClipboardWriterItem>>[];
    for (final item in items) {
      final representations = <raw.DataRepresentation>[];
      for (final data in item._data) {
        for (final entry in (await data).entries) {
          if (entry is EncodedDataEntrySimple) {
            representations.add(raw.DataRepresentation.simple(
              format: entry.format,
              data: entry.data,
            ));
          } else if (entry is EncodedDataEntryLazy) {
            representations.add(raw.DataRepresentation.lazy(
              format: entry.format,
              dataProvider: entry.dataProvider,
            ));
          } else {
            throw StateError("Invalid data entry type ${entry.runtimeType}");
          }
        }
      }
      if (representations.isNotEmpty) {
        providers.add(Pair(
          raw.DataProvider(representations: representations),
          item,
        ));
      }
    }
    final handles = <raw.DataProviderHandle>[];
    for (final p in providers) {
      final handle = await p.first.register();
      handles.add(handle);
      handle.onDispose.addListener(() {
        p.second._onDisposed.notify();
      });
      p.second._onRegistered.notify();
    }
    try {
      await raw.RawClipboardWriter.instance.write(handles);
    } catch (e) {
      for (final handle in handles) {
        handle.dispose();
      }
      rethrow;
    }
  }

  static final instance = ClipboardWriter._();
}
