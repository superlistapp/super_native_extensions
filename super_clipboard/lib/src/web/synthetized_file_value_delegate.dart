import 'dart:async';
import 'dart:typed_data';

import '../reader_value_delegate.dart';

/// Value delegate for values synthetized form file URIs
class SynthetizedFileValueDelegate extends DataReaderValueDelegate<Uint8List> {
  final Uri fileUri;

  SynthetizedFileValueDelegate._({
    required this.fileUri,
  });

  static Future<SynthetizedFileValueDelegate> withUri(Uri fileUri) async {
    throw UnsupportedError('Not supported on web');
  }

  @override
  Object? error;

  @override
  Uint8List? value;

  @override
  String? get fileName => fileUri.pathSegments.last;

  @override
  void onDispose() {}

  @override
  Future<bool> readNext() async {
    return false;
  }
}
