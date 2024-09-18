import 'data_provider.dart';

import 'native/clipboard_writer.dart'
    if (dart.library.js_interop) 'web/clipboard_writer.dart';

abstract class ClipboardWriter {
  static final ClipboardWriter instance = ClipboardWriterImpl();

  Future<void> write(List<DataProviderHandle> providers);
}
