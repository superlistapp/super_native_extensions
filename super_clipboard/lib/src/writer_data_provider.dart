import 'package:super_clipboard/src/util.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'writer.dart';

extension ClipboardWriterItemDataProvider on DataWriterItem {
  Future<raw.DataProvider> asDataProvider({
    String? suggestedName,
  }) async {
    final representations = <raw.DataRepresentation>[];
    for (final data in this.data) {
      for (final representation in (await data).representations) {
        representations.add(representation);
      }
    }
    return raw.DataProvider(
      representations: representations,
      suggestedName: suggestedName,
    );
  }

  Future<raw.DataProviderHandle> registerWithDataProvider(
      raw.DataProvider provider) async {
    final handle = await provider.register();
    handle.onDispose.addListener(() {
      (onDisposed as SimpleNotifier).notify();
    });
    (onRegistered as SimpleNotifier).notify();
    return handle;
  }
}
