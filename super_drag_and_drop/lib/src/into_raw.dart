import 'package:super_native_extensions/raw_clipboard.dart' as raw;
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
import 'package:super_clipboard/super_clipboard_internal.dart';

import 'drag_configuration.dart';
import 'indexed.dart';

extension DragImageIntoRaw on DragImage {
  Future<raw.DragImage> intoRaw(double devicePixelRatio) async {
    return raw.DragImage(
      imageData: await raw.ImageData.fromImage(image,
          devicePixelRatio: devicePixelRatio),
      sourceRect: sourceRect,
    );
  }
}

extension DragItemsIntoRaw on List<DragItem> {
  Future<List<raw.DragItem>> intoRaw(double devicePixelRatio) async {
    final providers = <raw.DataProvider>[];
    for (final item in this) {
      providers.add(await item.asDataProvider());
    }
    final handles = <raw.DataProviderHandle>[];
    for (final item in indexed()) {
      final handle =
          await item.value.registerWithDataProvider(providers[item.index]);
      handles.add(handle);
    }
    final items = <raw.DragItem>[];
    for (final item in indexed()) {
      items.add(raw.DragItem(
        dataProvider: handles[item.index],
        liftImage: await item.value.liftImage?.intoRaw(devicePixelRatio),
        image: await item.value.image.intoRaw(devicePixelRatio),
        localData: item.value.localData,
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
