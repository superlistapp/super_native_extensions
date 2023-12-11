import 'dart:async';

import '../data_provider.dart';
import '../data_provider_manager.dart';

class DataProviderManagerImpl extends DataProviderManager {
  @override
  FutureOr<DataProviderHandle> registerDataProvider(DataProvider provider) {
    return DataProviderHandle(0, provider);
  }

  @override
  FutureOr<void> unregisterDataProvider(int providerId) {}
}
