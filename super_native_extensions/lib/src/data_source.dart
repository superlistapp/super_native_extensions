import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:meta/meta.dart';

import 'data_source_manager.dart';
import 'util.dart';

/// Low level representation of data that can be written to clipboard or used
/// for drag&drop.
class DataSource {
  DataSource(this.items);

  dynamic serialize() => {
        'items': items.map((e) => e.serialize()),
      };

  final List<DataSourceItem> items;

  /// Registers this source with native code. The source data will be kept alive
  /// until handle is disposed.
  Future<DataSourceHandle> register() {
    return DataSourceManager.instance.registerDataSource(this);
  }
}

/// Single item of data source. Item can have multiple representation;
class DataSourceItem {
  DataSourceItem({
    required this.representations,
    this.suggestedName,
  });

  dynamic serialize() => {
        'representations': representations.map((e) => e.serialize()),
        'suggestedName': suggestedName,
      };

  final List<DataSourceItemRepresentation> representations;
  final String? suggestedName;
}

@sealed
abstract class DataSourceItemRepresentation {
  static DataSourceItemRepresentationSimple simple({
    required List<String> formats,
    required Object data,
  }) =>
      DataSourceItemRepresentationSimple._(
        formats: formats,
        data: data,
      );

  static DataSourceItemRepresentationLazy lazy({
    required List<String> formats,
    required FutureOr<Object> Function(String format) dataProvider,
  }) =>
      DataSourceItemRepresentationLazy._(
        formats: formats,
        dataProvider: dataProvider,
      );

  static DataSourceItemRepresentationVirtualFile virtualFile({
    required String format,
    required VirtualFileProvider virtualFileProvider,
    int? fileSize,
    VirtualFileStorage? storageSuggestion,
  }) =>
      DataSourceItemRepresentationVirtualFile._(
        format: format,
        virtualFileProvider: virtualFileProvider,
        fileSize: fileSize,
        storageSuggestion: storageSuggestion,
      );

  dynamic serialize();
}

/// Single representation of data source item. Useful when data is known upfront.
class DataSourceItemRepresentationSimple extends DataSourceItemRepresentation {
  DataSourceItemRepresentationSimple._({
    required this.formats,
    required this.data,
  });

  @override
  dynamic serialize() => {
        'type': 'simple',
        'formats': formats,
        'data': data,
      };

  /// List of platform-specific data formats.
  final List<String> formats;
  final Object data;
}

/// Single reprsentation of data source item. Useful when data is generated
/// on demand.
class DataSourceItemRepresentationLazy extends DataSourceItemRepresentation {
  DataSourceItemRepresentationLazy._({
    required this.formats,
    required this.dataProvider,
  }) : id = _nextId++;

  @override
  dynamic serialize() => {
        'type': 'lazy',
        'id': id,
        'formats': formats,
      };

  final int id;
  final List<String> formats;
  final FutureOr<Object> Function(String format) dataProvider;
}

class Progress {
  Progress(this.onCancel, ValueNotifier<int> onProgress)
      : _onProgress = onProgress;

  /// Updates operation progress. Range is 0 to 100.
  void updateProgress(int progress) {
    _onProgress.value = progress;
  }

  final Listenable onCancel;
  final ValueNotifier<int> _onProgress;
}

typedef VirtualFileProvider = void Function(EventSink sink, Progress progress);

enum VirtualFileStorage { temporaryFile, memory }

class DataSourceItemRepresentationVirtualFile
    extends DataSourceItemRepresentation {
  DataSourceItemRepresentationVirtualFile._({
    required this.format,
    required this.virtualFileProvider,
    this.fileSize,
    this.storageSuggestion,
  }) : id = _nextId++;

  @override
  serialize() => {
        'type': 'virtualFile',
        'id': id,
        'format': format,
        'fileSize': fileSize,
        'storageSuggestion': storageSuggestion?.name,
      };

  final int id;
  final String format;
  final VirtualFileProvider virtualFileProvider;
  final int? fileSize;
  final VirtualFileStorage? storageSuggestion;
}

class DataSourceHandle {
  DataSourceHandle(this.id, this.source);

  final int id;
  final DataSource source;
  Listenable get onDispose => _onDispose;

  final _onDispose = SimpleNotifier();

  bool _disposed = false;

  Future<void> dispose() async {
    assert(!_disposed);
    _disposed = true;
    _onDispose.notify();
    await DataSourceManager.instance.unregisterDataSource(id);
  }
}

int _nextId = 1;
