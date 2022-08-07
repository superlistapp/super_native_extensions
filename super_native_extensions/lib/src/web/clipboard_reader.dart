import '../clipboard_reader.dart';
import '../reader.dart';
import '../reader_manager.dart';
import 'clipboard_api.dart';
import 'reader_manager.dart';

class ClipboardReaderHandle extends DataReaderItemHandleImpl {
  ClipboardReaderHandle(this.item);

  final ClipboardItem item;

  @override
  Future<List<String>> getFormats() async {
    return item.types.toList(growable: false);
  }

  @override
  Future<Object?> getDataForFormat(String format) async {
    final data = await item.getType(format);
    if (format.startsWith('text/')) {
      return data.text();
    } else {
      return (await data.arrayBuffer())?.asUint8List();
    }
  }

  @override
  Future<String?> suggestedName() async {
    // ClipboardItem can tell that it is an attachment but can not
    // provide name. Go figure.
    return null;
  }
}

class ClipboardReaderImpl extends ClipboardReader {
  @override
  Future<DataReader> newClipboardReader() async {
    final items = await getClipboard().read();
    final handle = DataReaderHandleImpl(
      items.map((e) => ClipboardReaderHandle(e)).toList(growable: false),
    );

    return DataReader(handle: handle as DataReaderHandle);
  }
}
