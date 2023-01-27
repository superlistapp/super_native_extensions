import 'dart:async';
import 'dart:typed_data';

import 'reader.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

Future<Uint8List> _readAll(Stream<Uint8List> stream) {
  final completer = Completer<Uint8List>();
  final builder = BytesBuilder(copy: false);
  stream.listen((event) {
    builder.add(event);
  }, onDone: () {
    completer.complete(builder.takeBytes());
  }, onError: (e) {
    completer.completeError(e);
  });
  return completer.future;
}

class DataReaderFileValueAdapter extends DataReaderFile {
  DataReaderFileValueAdapter(this.value);

  final Uint8List value;

  @override
  void close() {}

  @override
  String? get fileName => null;

  @override
  int? get fileSize => value.length;

  @override
  Stream<Uint8List> getStream() {
    return Stream.value(value);
  }

  @override
  Future<Uint8List> readAll() async {
    return value;
  }
}

class DataReaderVirtualFileAdapter extends DataReaderFile {
  DataReaderVirtualFileAdapter(this.file);

  bool _disposed = false;
  bool _streamRequested = false;

  final raw.VirtualFile file;

  @override
  void close() {
    if (!_disposed) {
      _disposed = true;
      file.close();
    }
  }

  void maybeClose() {
    if (_streamRequested) {
      return;
    }
    close();
  }

  @override
  String? get fileName => file.fileName;

  @override
  int? get fileSize => file.length;

  @override
  Stream<Uint8List> getStream() {
    if (_streamRequested) {
      throw StateError('Stream already requested');
    }
    if (_disposed) {
      throw StateError('Already disposed');
    }
    _streamRequested = true;
    return _getStream();
  }

  Stream<Uint8List> _getStream() async* {
    try {
      while (true) {
        final next = await file.readNext();
        if (next.isEmpty) {
          break;
        } else {
          yield next;
        }
      }
    } finally {
      _streamRequested = false;
      close();
    }
  }

  @override
  Future<Uint8List> readAll() async {
    return _readAll(getStream());
  }
}
