import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'clipboard_api.dart';

import 'js_interop.dart';
import 'reader.dart';
import 'reader_manager.dart';

class ClipboardReaderImpl extends ClipboardReader {
  @override
  Future<DataReader> newClipboardReader() async {
    final items = await getClipboard().read();
    final handle = $DataReaderHandle(
      items
          .map(
            (e) => ClipboardItemHandle(e),
          )
          .toList(growable: false),
    );

    return DataReader(handle: handle as DataReaderHandle);
  }

  @override
  bool get available => clipboardItemAvailable;
}
