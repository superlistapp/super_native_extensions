import 'dart:html';
import 'dart:js_util';

import '../clipboard_writer.dart';
import '../data_provider.dart';
import 'clipboard_api.dart';
import 'js_interop.dart';

class ClipboardWriterImpl extends ClipboardWriter {
  List<DataProviderHandle> _currentPayload = [];

  ClipboardItem translateProvider(DataProvider provider) {
    final representations = <String, Promise<Blob>>{};
    for (final repr in provider.representations) {
      if (repr.format == 'text/uri-list') {
        // Writing URI list to clipboard on web is not supported
        continue;
      }
      if (repr is DataRepresentationSimple) {
        representations[repr.format] =
            futureToPromise((() async => Blob([repr.data], repr.format))());
      } else if (repr is DataRepresentationLazy) {
        representations[repr.format] = futureToPromise(
            (() async => Blob([await repr.dataProvider()], repr.format))());
      }
    }
    return ClipboardItem(jsify(representations));
  }

  @override
  Future<void> write(List<DataProviderHandle> providers) async {
    for (final handle in _currentPayload) {
      await handle.dispose();
    }
    _currentPayload = providers;
    final clipboard = getClipboard();
    final items = providers.map((e) => translateProvider(e.provider));
    await clipboard.write(items);
  }
}
