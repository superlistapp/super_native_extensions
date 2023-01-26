import 'dart:async';
import 'dart:io';
import 'dart:typed_data';

import '../reader_value_delegate.dart';

/// Value delegate for values synthetized from file URIs
class SynthetizedFileValueDelegate extends DataReaderValueDelegate<Uint8List> {
  final Uri fileUri;

  SynthetizedFileValueDelegate._({
    required this.fileUri,
  });

  static Future<SynthetizedFileValueDelegate> withUri(Uri fileUri) async {
    final res = SynthetizedFileValueDelegate._(fileUri: fileUri);
    await res.readNext();
    return res;
  }

  @override
  Object? error;

  @override
  Uint8List? value;

  @override
  String? get fileName => fileUri.pathSegments.last;

  StreamIterator<List<int>>? _iterator;

  @override
  void onDispose() {
    _iterator?.cancel();
  }

  @override
  Future<bool> readNext() async {
    if (_iterator == null) {
      final file = File(fileUri.toFilePath());
      _iterator = StreamIterator(file.openRead());
    }

    try {
      final res = (await _iterator?.moveNext()) ?? false;
      if (res) {
        value = _iterator!.current as Uint8List;
      }
      return res;
    } catch (e) {
      value = null;
      error = e;
      return false;
    }
  }
}
