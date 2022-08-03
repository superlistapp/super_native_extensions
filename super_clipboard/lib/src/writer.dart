import 'dart:async';

import 'package:flutter/material.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'encoded_data.dart';
import 'util.dart';
import 'format.dart';
import 'writer_data_provider.dart';

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

  List<FutureOr<EncodedData>> get data => _data;
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
      final provider = await item.asDataProvider();
      if (provider.representations.isNotEmpty) {
        providers.add(Pair(provider, item));
      }
    }
    final handles = <raw.DataProviderHandle>[];
    for (final p in providers) {
      handles.add(await p.second.registerWithDataProvider(p.first));
    }
    try {
      await raw.ClipboardWriter.instance.write(handles);
    } catch (e) {
      for (final handle in handles) {
        handle.dispose();
      }
      rethrow;
    }
  }

  static final instance = ClipboardWriter._();
}
