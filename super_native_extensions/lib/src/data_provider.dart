import 'dart:async';

import 'package:flutter/foundation.dart';

import 'data_provider_manager.dart';
import 'util.dart';
import 'clipboard_writer.dart';
import 'drag.dart';

class DataProvider {
  DataProvider({
    required this.representations,
    this.suggestedName,
  });

  /// Registers this source with native code. The source data will be kept alive
  /// until handle is disposed.
  FutureOr<DataProviderHandle> register() {
    return DataProviderManager.instance.registerDataProvider(this);
  }

  final List<DataRepresentation> representations;
  final String? suggestedName;
}

sealed class DataRepresentation {
  static DataRepresentationSimple simple({
    required String format,
    required Object? data,
  }) =>
      DataRepresentationSimple._(
        format: format,
        data: data,
      );

  static DataRepresentationLazy lazy({
    required String format,
    required FutureOr<Object?> Function() dataProvider,
  }) =>
      DataRepresentationLazy._(
        format: format,
        dataProvider: dataProvider,
      );

  static DataRepresentationVirtualFile virtualFile({
    required String format,
    required VirtualFileProvider virtualFileProvider,
    VirtualFileStorage? storageSuggestion,
  }) =>
      DataRepresentationVirtualFile._(
        format: format,
        virtualFileProvider: virtualFileProvider,
        storageSuggestion: storageSuggestion,
      );

  String get format;
  dynamic serialize();
}

/// Single representation of data source item. Useful when data is known upfront.
class DataRepresentationSimple extends DataRepresentation {
  DataRepresentationSimple._({
    required this.format,
    required this.data,
  });

  @override
  dynamic serialize() => {
        'type': 'simple',
        'format': format,
        'data': data,
      };

  /// List of platform-specific data formats.
  @override
  final String format;
  final Object? data;
}

/// Single representation of data source item. Useful when data is generated
/// on demand.
class DataRepresentationLazy extends DataRepresentation {
  DataRepresentationLazy._({
    required this.format,
    required this.dataProvider,
  }) : id = _nextId++;

  @override
  dynamic serialize() => {
        'type': 'lazy',
        'id': id,
        'format': format,
      };

  final int id;
  @override
  final String format;
  final FutureOr<Object?> Function() dataProvider;
}

/// Progress of a write operation.
abstract class WriteProgress {
  /// Manually updates progress of a write operation. If not called,
  /// progress is determined automatically based on total size and size written.
  void updateProgress(double fraction);

  /// Listenable invoked when receiver cancels the write operation.
  Listenable get onCancel;
}

/// Returns stream sink for virtual file. File size must be provided before
/// being able to write any content.
typedef VirtualFileEventSinkProvider = EventSink Function(
    {required int fileSize});

/// Callback invoked when receiver requests a virtual file.
typedef VirtualFileProvider = void Function(
    VirtualFileEventSinkProvider sinkProvider, WriteProgress progress);

/// Preferred storage used when writing virtual file.
enum VirtualFileStorage { temporaryFile, memory }

class DataRepresentationVirtualFile extends DataRepresentation {
  DataRepresentationVirtualFile._({
    required this.format,
    required this.virtualFileProvider,
    this.storageSuggestion,
  }) : id = _nextId++;

  @override
  serialize() => {
        'type': 'virtualFile',
        'id': id,
        'format': format,
        'storageSuggestion': storageSuggestion?.name,
      };

  final int id;
  @override
  final String format;
  final VirtualFileProvider virtualFileProvider;

  final VirtualFileStorage? storageSuggestion;
}

int _nextId = 1;

class DataProviderHandle {
  DataProviderHandle(this.id, this.provider);

  final int id;
  final DataProvider provider;
  Listenable get onDispose => _onDispose;

  final _onDispose = SimpleNotifier();

  bool _disposed = false;

  /// Disposes the data source. This should not be called directly.
  /// DataSource is disposed automatically when no longer needed for clipboard
  /// or drag and drop by [ClipboardWriter] and [DragContext].
  Future<void> dispose() async {
    if (!_disposed) {
      _disposed = true;
      _onDispose.notify();
      _onDispose.dispose();
      await DataProviderManager.instance.unregisterDataProvider(id);
    }
  }
}
