import 'dart:async';

import 'package:flutter/material.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'encoded_data.dart';
import 'util.dart';
import 'format.dart';

/// Represents a single item in the clipboard. The item can have multiple
/// renditions (each represented as entries in [EncodedData]).
/// To get encoded data for values use [EncodableDataFormat.encode];
class ClipboardWriterItem {
  void addData(FutureOr<EncodedData> data) {
    _data.add(data);
  }

  /// Invoked when the item is sucessfully registered with native code.
  Listenable get onRegistered => _onRegistered;

  /// Called when the native code is done with the item and the data is
  /// no longer needed. Only guaranteed to be called if [onRegistered] has
  /// been called before.
  Listenable get onDisposed => _onDisposed;

  final _onRegistered = SimpleNotifier();
  final _onDisposed = SimpleNotifier();
  final _data = <FutureOr<EncodedData>>[];
}

/// Example for using clipboard writer:
/// ```dart
/// final item = ClipboardWriterItem();
/// item.addData(formatHtml.encode('<b><i>Html</i></b> Value'));
/// item.addData(formatPlainText.encodeLazy(() =>
///                                   'Plaintext value resolved lazily'));
/// ClipboardWriter.instance.write([item]);
/// ```
class ClipboardWriter {
  ClipboardWriter._();

  /// Writes the provided items in system clipboard.
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
