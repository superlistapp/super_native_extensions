import 'dart:async';

import 'data_provider.dart';

import 'native/data_provider_manager.dart'
    if (dart.library.js_interop) 'web/data_provider_manager.dart';

abstract class DataProviderManager {
  static final DataProviderManager instance = DataProviderManagerImpl();

  FutureOr<DataProviderHandle> registerDataProvider(DataProvider provider);
  FutureOr<void> unregisterDataProvider(int providerId);
}
