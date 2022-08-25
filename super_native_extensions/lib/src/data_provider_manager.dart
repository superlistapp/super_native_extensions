import 'data_provider.dart';

import 'native/data_provider_manager.dart'
    if (dart.library.js) 'web/data_provider_manager.dart';

abstract class DataProviderManager {
  static final DataProviderManager instance = DataProviderManagerImpl();

  Future<DataProviderHandle> registerDataProvider(DataProvider provider);
  Future<void> unregisterDataProvider(int providerId);
}
