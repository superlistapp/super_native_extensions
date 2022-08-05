import 'package:super_clipboard/src/util.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;
import 'encoded_data.dart';
import 'writer.dart';

extension ClipboardWriterItemDataProvider on DataWriterItem {
  Future<raw.DataProvider> asDataProvider({
    String? suggestedName,
  }) async {
    final representations = <raw.DataRepresentation>[];
    for (final data in this.data) {
      for (final entry in (await data).entries) {
        if (entry is EncodedDataEntrySimple) {
          representations.add(raw.DataRepresentation.simple(
            format: entry.format,
            data: entry.data,
          ));
        } else if (entry is EncodedDataEntryLazy) {
          representations.add(raw.DataRepresentation.lazy(
            format: entry.format,
            dataProvider: entry.dataProvider,
          ));
        } else {
          throw StateError("Invalid data entry type ${entry.runtimeType}");
        }
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
