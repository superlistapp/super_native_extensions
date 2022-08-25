import '../data_provider.dart';
import '../data_provider_manager.dart';

class DataProviderManagerImpl extends DataProviderManager {
  @override
  Future<DataProviderHandle> registerDataProvider(DataProvider provider) async {
    return DataProviderHandle(0, provider);
  }

  @override
  Future<void> unregisterDataProvider(int providerId) async {}
}
