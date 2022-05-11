#import "SuperNativeExtensionsPlugin.h"

extern void super_native_extensions_init(void);

@implementation SuperNativeExtensionsPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar>*)registrar {
  SuperNativeExtensionsPlugin* instance = [[SuperNativeExtensionsPlugin alloc] init];
  (void)instance;
  super_native_extensions_init();
}

@end
