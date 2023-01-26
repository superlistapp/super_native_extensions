import 'dart:async';
import 'dart:typed_data';

import 'reader.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

export 'native/synthetized_file_value_delegate.dart'
    if (dart.library.js) 'web/synthetized_file_value_delegate.dart';

abstract class DataReaderValueDelegate<T extends Object> {
  T? get value;
  Object? get error;
  Future<bool> readNext();
  String? get fileName;

  void onDispose();

  void sendAsValue(AsyncValueChanged<DataReaderValue<T>> callback) {
    final res = callback(DataReaderValue(this));
    if (res is Future) {
      res.then((_) => dispose());
    } else {
      dispose();
    }
  }

  bool _disposed = false;
  bool _streamRequested = false;
  bool _streamActive = false;
  void dispose({bool force = false}) {
    if (_streamRequested) {
      // Will be disposed when stream is closed.
      return;
    }
    if (force && _streamRequested && !_streamActive) {
      _streamRequested = false;
    }
    if (!_disposed) {
      _disposed = true;
      onDispose();
    }
  }
}

extension DataReaderValueDelegateUint8List
    on DataReaderValueDelegate<Uint8List> {
  Stream<Uint8List> asStream() {
    if (_streamRequested) {
      throw StateError('Stream already requested');
    }
    if (_disposed) {
      throw StateError('Already disposed');
    }
    _streamRequested = true;
    return _asStream();
  }

  Stream<Uint8List> _asStream() async* {
    try {
      _streamActive = true;
      while (true) {
        if (error != null) {
          throw error!;
        }
        if (value != null) {
          yield value!;
        }
        if (!await readNext()) {
          break;
        }
      }
    } finally {
      _streamActive = false;
      _streamRequested = false;
      dispose();
    }
  }

  Future<Uint8List> readAll() async {
    if (_disposed) {
      throw StateError('Already disposed');
    }
    try {
      final buffer = BytesBuilder();
      while (true) {
        if (error != null) {
          throw error!;
        }
        final chunk = value;
        if (chunk == null) {
          break;
        }
        buffer.add(chunk);
        if (!await readNext()) {
          break;
        }
      }
      return buffer.takeBytes();
    } finally {
      dispose();
    }
  }
}

class SimpleValueDelegate<T extends Object> extends DataReaderValueDelegate<T> {
  SimpleValueDelegate({
    this.value,
    this.error,
  });

  @override
  final T? value;

  @override
  final Object? error;

  @override
  String? get fileName => null;

  @override
  void onDispose() {}

  @override
  Future<bool> readNext() {
    return Future.value(false);
  }
}

/// Delegate for virtual files.
class VirtualFileValueDelegate extends DataReaderValueDelegate<Uint8List> {
  VirtualFileValueDelegate._(this.virtualFile);

  static Future<VirtualFileValueDelegate> fromFile(raw.VirtualFile file) async {
    final res = VirtualFileValueDelegate._(file);
    await res.readNext();
    return res;
  }

  Object? _error;
  Uint8List? _buffer;

  @override
  Uint8List? get value => _buffer;

  @override
  Object? get error => _error;

  @override
  String? get fileName => virtualFile.fileName;

  @override
  void onDispose() {
    virtualFile.close();
  }

  @override
  Future<bool> readNext() async {
    try {
      _buffer = await virtualFile.readNext();
    } catch (e) {
      _error = e;
      _buffer = null;
    }
    return _buffer?.isNotEmpty == true;
  }

  final raw.VirtualFile virtualFile;
}
