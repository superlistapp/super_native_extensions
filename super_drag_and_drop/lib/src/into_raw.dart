import 'package:super_native_extensions/raw_clipboard.dart' as raw;
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
import 'package:super_clipboard/super_clipboard_internal.dart';

import 'drag_configuration.dart';

extension DragItemsIntoRaw on List<DragConfigurationItem> {
  Future<List<raw.DragItem>> intoRaw(double devicePixelRatio) async {
    final providers = <raw.DataProvider>[];
    for (final item in this) {
      providers.add(await item.item.asDataProvider());
    }
    final handles = <raw.DataProviderHandle>[];
    for (final (index, item) in indexed) {
      final handle = await item.item.registerWithDataProvider(providers[index]);
      handles.add(handle);
    }
    final items = <raw.DragItem>[];
    for (final (index, item) in indexed) {
      items.add(raw.DragItem(
        dataProvider: handles[index],
        image: item.image,
        liftImage: item.liftImage,
        localData: item.item.localData,
      ));
    }
    return items;
  }
}

extension DragConfigurationIntoRaw on DragConfiguration {
  Future<raw.DragConfiguration> intoRaw(double devicePixelRatio) async {
    return raw.DragConfiguration(
      allowedOperations: allowedOperations,
      items: await items.intoRaw(devicePixelRatio),
      animatesToStartingPositionOnCancelOrFail:
          options.animatesToStartingPositionOnCancelOrFail,
      prefersFullSizePreviews: options.prefersFullSizePreviews,
    );
  }
}
