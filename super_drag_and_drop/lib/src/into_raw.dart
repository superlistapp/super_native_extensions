import 'package:super_native_extensions/raw_clipboard.dart' as raw;
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;
import 'package:super_clipboard/super_clipboard_internal.dart';

import 'drag_configuration.dart';
import 'indexed.dart';

extension DragImageIntoRaw on DragImage {
  Future<raw.DragImageData> intoRaw(double devicePixelRatio) async {
    return raw.DragImageData(
      image: await image.intoRaw(devicePixelRatio),
      imageSource: image,
      liftImage: await liftImage?.intoRaw(devicePixelRatio),
      liftImageSource: liftImage,
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
      final image = item.value.image is Future
          ? await item.value.image
          : item.value.image as DragImage;
      items.add(raw.DragItem(
        dataProvider: handles[item.index],
        image: await image.intoRaw(devicePixelRatio),
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
