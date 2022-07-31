import 'data_provider.dart';

import 'native/clipboard_writer.dart'
    if (dart.library.js) 'web/clipboard_writer.dart';

abstract class RawClipboardWriter {
  static final RawClipboardWriter instance = RawClipboardWriterImpl();

  Future<void> write(List<DataProviderHandle> providers);
}
