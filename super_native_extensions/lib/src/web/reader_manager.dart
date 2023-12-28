import 'dart:async';

import 'package:flutter/foundation.dart';

import '../reader.dart';
import '../reader_manager.dart';

/// Unlike native implementation, item handle on web contains the actual implementation and
/// [ReaderManagerImpl] merely forwards calls to the handle.
abstract class $DataReaderItemHandle {
  Future<List<String>> getFormats();
  Future<Object?> getDataForFormat(String format);
  Future<String?> suggestedName();
  Future<bool> canGetVirtualFile(String format);
  Future<VirtualFileReceiver?> createVirtualFileReceiver(
    DataReaderItemHandle handle, {
    required String format,
  });
}

class SimpleProgress extends ReadProgress {
  @override
  void cancel() {}

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  @override
  ValueListenable<double?> get fraction => _fraction;

  void done() {
    _fraction.value = 1.0;
  }

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}

class $DataReaderHandle {
  $DataReaderHandle(this.items);
  final List<$DataReaderItemHandle> items;
}

/// ReaderManagerImpl on web forwards calls to underlying handles.
class ReaderManagerImpl extends ReaderManager {
  @override
  Future<void> dispose(DataReaderHandle reader) async {
    // We don't register the items anywhere so there's nothing to unregister.
  }

  @override
  (Future<Object?>, ReadProgress) getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final impl = handle as $DataReaderItemHandle;
    final progress = SimpleProgress();
    final res = impl.getDataForFormat(format);
    final completer = Completer<Object?>();
    res.then((value) {
      progress.done();
      completer.complete(value);
    }).catchError((error) {
      progress.done();
      completer.completeError(error);
    });
    return (completer.future, progress);
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) {
    final impl = handle as $DataReaderItemHandle;
    return impl.getFormats();
  }

  @override
  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader) async {
    final handle = reader as $DataReaderHandle;
    return handle.items.map((e) => e as DataReaderItemHandle).toList();
  }

  @override
  Future<List<DataReaderItemInfo>> getItemInfo(
    Iterable<DataReaderItemHandle> handles, {
    Duration? timeout,
  }) async {
    final res = <DataReaderItemInfo>[];
    final stopwatch = Stopwatch()..start();
    for (final handle in handles) {
      final impl = handle as $DataReaderItemHandle;
      final formats = await impl.getFormats();
      final receivers = <VirtualFileReceiver>[];
      for (final format in formats) {
        if (await impl.canGetVirtualFile(format)) {
          final receiver =
              await impl.createVirtualFileReceiver(handle, format: format);
          if (receiver != null) {
            receivers.add(receiver);
          }
        }
      }
      final info = DataReaderItemInfo(
        handle,
        formats: formats,
        synthesizedFormats: [],
        virtualReceivers: receivers,
        suggestedName: await impl.suggestedName(),
        synthesizedFromURIFormat: null,
      );
      res.add(info);
      if (timeout != null && stopwatch.elapsed > timeout) {
        break;
      }
    }
    return res;
  }

  @override
  VirtualFile createVirtualFileFromUri(Uri uri) {
    throw UnsupportedError('createVirtualFileFromUri is not supported on web');
  }
}
