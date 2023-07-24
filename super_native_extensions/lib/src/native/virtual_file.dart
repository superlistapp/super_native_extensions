import 'dart:io';
import 'package:path/path.dart' as path;

import 'package:flutter/foundation.dart';
import 'package:uuid/uuid.dart';

import '../reader.dart';

class VirtualFileFromFile extends VirtualFile {
  VirtualFileFromFile({
    required this.file,
    required this.onClose,
  });

  final VoidCallback onClose;
  final File file;
  RandomAccessFile? _file;

  @override
  void close() async {
    try {
      _file?.closeSync();
    } catch (_) {}
    onClose();
  }

  @override
  String? get fileName => path.basename(file.path);

  @override
  int? get length => file.existsSync() ? file.lengthSync() : null;

  @override
  Future<Uint8List> readNext() async {
    _file ??= await file.open();
    return _file!.read(1024 * 256);
  }
}

/// Virtual file receiver implementation that works on provided copy.
abstract class CopyVirtualFileReceiver extends VirtualFileReceiver {
  @override
  (Future<VirtualFile>, ReadProgress) receiveVirtualFile() {
    final uuid = const Uuid().v4().toString();
    final folder = path.join(Directory.systemTemp.path, 'vfr-$uuid');
    Directory(folder).createSync();
    try {
      final (path, progress) = copyVirtualFile(targetFolder: folder);
      final future = path.then((value) {
        return VirtualFileFromFile(
          file: File(value),
          onClose: () {
            Directory(folder).deleteSync(recursive: true);
          },
        );
      }, onError: (e) {
        Directory(folder).deleteSync(recursive: true);
        throw e;
      });
      return (future, progress);
    } catch (e) {
      Directory(folder).deleteSync(recursive: true);
      rethrow;
    }
  }
}
