import 'package:super_native_extensions/src/data_provider.dart';
import 'package:super_native_extensions/src/data_provider_manager.dart';

class DataProviderManagerImpl extends DataProviderManager {
  @override
  Future<DataProviderHandle> registerDataProvider(DataProvider provider) async {
    return DataProviderHandle(0, provider);
  }

  @override
  Future<void> unregisterDataProvider(int providerId) async {}
}
