import 'dart:async';

import 'package:flutter/foundation.dart';

import '../data_provider.dart';
import '../reader.dart';
import '../reader_manager.dart';

class SimpleProgress extends ReadProgress {
  @override
  void cancel() {}

  @override
  ValueListenable<bool> get cancellable => _cancellable;

  @override
  ValueListenable<double?> get fraction => _fraction;

  final _cancellable = ValueNotifier(false);
  final _fraction = ValueNotifier<double?>(null);
}

class DataReaderHandleImpl {
  DataReaderHandleImpl(this.items);
  final List<DataReaderItemHandleImpl> items;
}

abstract class DataReaderItemHandleImpl {
  Future<List<String>> getFormats();
  Future<Object?> getDataForFormat(String format);
  Future<String?> suggestedName();
}

class DataProviderReaderItem extends DataReaderItemHandleImpl {
  DataProviderReaderItem(this.provider);

  final DataProviderHandle provider;

  @override
  Future<Object?> getDataForFormat(String format) async {
    for (final representation in provider.provider.representations) {
      if (representation is DataRepresentationSimple) {
        if (representation.format == format) {
          return representation.data;
        }
      } else if (representation is DataRepresentationLazy) {
        if (representation.format == format) {
          return representation.dataProvider();
        }
      }
    }
    return null;
  }

  @override
  Future<List<String>> getFormats() async {
    return provider.provider.representations
        .map((e) => e.format)
        .toList(growable: false);
  }

  @override
  Future<String?> suggestedName() async {
    return provider.provider.suggestedName;
  }
}

class ReaderManagerImpl extends ReaderManager {
  @override
  Future<void> dispose(DataReaderHandle reader) async {
    // we don't register the items anywhere so there's nothing to undergister
  }

  @override
  Pair<Future<Object?>, ReadProgress> getItemData(
    DataReaderItemHandle handle, {
    required String format,
  }) {
    final impl = handle as DataReaderItemHandleImpl;
    final progress = SimpleProgress();
    final res = impl.getDataForFormat(format);
    final completer = Completer<Object?>();
    res.then((value) {
      progress._fraction.value = 1.0;
      completer.complete(value);
    }).catchError((error) {
      progress._fraction.value = 1.0;
      completer.completeError(error);
    });
    return Pair(completer.future, progress);
  }

  @override
  Future<List<String>> getItemFormats(DataReaderItemHandle handle) {
    final impl = handle as DataReaderItemHandleImpl;
    return impl.getFormats();
  }

  @override
  Future<String?> getItemSuggestedName(DataReaderItemHandle handle) {
    final impl = handle as DataReaderItemHandleImpl;
    return impl.suggestedName();
  }

  @override
  Future<bool> isFormatSynthetized(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return false;
  }

  @override
  Future<List<DataReaderItemHandle>> getItems(DataReaderHandle reader) async {
    final handle = reader as DataReaderHandleImpl;
    return handle.items.map((e) => e as DataReaderItemHandle).toList();
  }

  @override
  Future<bool> canGetVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
  }) async {
    return false;
  }

  @override
  Pair<Future<String?>, ReadProgress> getVirtualFile(
    DataReaderItemHandle handle, {
    required String format,
    required String targetFolder,
  }) {
    throw UnsupportedError('Virtual files are not supported on web');
  }
}
