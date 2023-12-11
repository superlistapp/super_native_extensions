import 'dart:async';

import 'package:super_clipboard/src/future_util.dart';
import 'package:super_clipboard/src/util.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'writer.dart';

extension ClipboardWriterItemDataProvider on DataWriterItem {
  FutureOr<raw.DataProvider> asDataProvider() {
    final representations = <raw.DataRepresentation>[];
    for (final data in this.data) {
      if (data is Future) {
        return _asDataProviderAsync();
      }
      for (final representation in data.representations) {
        representations.add(representation);
      }
    }
    return raw.DataProvider(
      representations: representations,
      suggestedName: suggestedName,
    );
  }

  Future<raw.DataProvider> _asDataProviderAsync() async {
    final representations = <raw.DataRepresentation>[];
    for (final data in this.data) {
      for (final representation in (await data).representations) {
        representations.add(representation);
      }
    }
    return raw.DataProvider(
      representations: representations,
      suggestedName: suggestedName,
    );
  }

  FutureOr<raw.DataProviderHandle> registerWithDataProvider(
    raw.DataProvider provider,
  ) {
    final handle = provider.register();
    return handle.then((handle) {
      final onDisposed = this.onDisposed as SimpleNotifier;
      final onRegistered = this.onRegistered as SimpleNotifier;
      handle.onDispose.addListener(() {
        onDisposed.notify();
        onDisposed.dispose();
        onRegistered.dispose();
      });
      onRegistered.notify();
      return handle;
    });
  }
}

extension DataWriterItemListExt on Iterable<DataWriterItem> {
  // Transforms list of data writer items into a list of data provider handles
  // that can be used to interface with raw clipboard API.
  Future<void> withHandles(
    Future<void> Function(List<raw.DataProviderHandle>) callback,
  ) async {
    final providers = <(raw.DataProvider, DataWriterItem)>[];
    for (final item in this) {
      final provider = await item.asDataProvider();
      if (provider.representations.isNotEmpty) {
        providers.add((provider, item));
      }
    }
    final handles = <raw.DataProviderHandle>[];
    for (final (provider, writer) in providers) {
      handles.add(await writer.registerWithDataProvider(provider));
    }
    try {
      await callback(handles);
    } catch (e) {
      for (final handle in handles) {
        handle.dispose();
      }
      rethrow;
    }
  }

  // Transforms list of data writer item into a list of data provider handles
  // synchronously. This is used when interfacing with HTML copy event, which
  // requires synchronous handling. The call will fail if any of the data
  // providers are async.
  void withHandlesSync(void Function(List<raw.DataProviderHandle>) callback) {
    final providers = <(raw.DataProvider, DataWriterItem)>[];
    for (final item in this) {
      final provider = item.asDataProvider();
      if (provider is Future) {
        // Firefox will throw exception when trying to set clipboard event
        // data outside DOM event handler.
        throw UnsupportedError(
            'Cannot use asynchronous data provider in current context. '
            'HTML clipboard events only support setting data synchronously.');
      }
      if (provider.representations.isNotEmpty) {
        providers.add((provider, item));
      }
    }
    final handles = <raw.DataProviderHandle>[];
    for (final (provider, writer) in providers) {
      final handle = writer.registerWithDataProvider(provider);
      if (handle is Future) {
        throw StateError(
            'Data provider registration returned a future. This is not expected in sync context.');
      }
      handles.add(handle);
    }
    try {
      callback(handles);
    } catch (e) {
      for (final handle in handles) {
        handle.dispose();
      }
      rethrow;
    }
  }
}
