#import "SuperNativeExtensionsPlugin.h"

extern void super_native_extensions_init(void);

// Flutter API doesn't provide an official way to get a view from registrar.
// This will likely break in future when multiple views per engine are
// supported. But that will be a major breaking change anyway.
@interface _FlutterPluginRegistrar : NSObject
@property(readwrite, nonatomic) FlutterEngine *flutterEngine;
@end

@interface SuperNativeExtensionsPlugin () {
  __weak FlutterEngine *engine;
}
@end

@implementation SuperNativeExtensionsPlugin
+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {
  SuperNativeExtensionsPlugin *instance =
      [[SuperNativeExtensionsPlugin alloc] init];

  FlutterMethodChannel *channel =
      [FlutterMethodChannel methodChannelWithName:@"super_native_extensions"
                                  binaryMessenger:registrar.messenger];
  [registrar addMethodCallDelegate:instance channel:channel];

  instance->engine = ((_FlutterPluginRegistrar *)registrar).flutterEngine;
  super_native_extensions_init();
}

- (void)handleMethodCall:(FlutterMethodCall *)call
                  result:(FlutterResult)result {
  if ([call.method isEqual:@"getFlutterView"]) {
    result(@((uintptr_t)engine.viewController.view));
  } else {
    result(FlutterMethodNotImplemented);
  }
}

@end
