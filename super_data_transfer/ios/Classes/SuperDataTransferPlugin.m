#import "SuperDataTransferPlugin.h"

extern void super_data_transfer_init(void);

@implementation SuperDataTransferPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar>*)registrar {
  SuperDataTransferPlugin* instance = [[SuperDataTransferPlugin alloc] init];
  (void)instance;
  super_data_transfer_init();
}

@end
