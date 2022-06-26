#import "SuperNativeExtensionsPlugin.h"

extern void super_native_extensions_init(void);

@interface SuperNativeExtensionsPlugin () {
  __weak NSView *flutterView;
}
@end

@implementation SuperNativeExtensionsPlugin

+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {

  SuperNativeExtensionsPlugin *instance =
      [[SuperNativeExtensionsPlugin alloc] init];
  // View is available only after registerWithRegistrar: completes. And we don't
  // want to keep strong reference to the registrar in instance because it
  // references engine and unfortunately instance itself will leak given current
  // Flutter plugin architecture on macOS;
  dispatch_async(dispatch_get_main_queue(), ^{
    instance->flutterView = registrar.view;
  });
  FlutterMethodChannel *channel =
      [FlutterMethodChannel methodChannelWithName:@"super_native_extensions"
                                  binaryMessenger:registrar.messenger];
  [registrar addMethodCallDelegate:instance channel:channel];
  super_native_extensions_init();
}

- (void)handleMethodCall:(FlutterMethodCall *)call
                  result:(FlutterResult)result {
  if ([call.method isEqual:@"getFlutterView"]) {
    result(@((uintptr_t)flutterView));
  } else {
    result(FlutterMethodNotImplemented);
  }
}

@end
