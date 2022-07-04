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

API_AVAILABLE(macos(10.12))
@interface SNEForwardingFilePromiseProvider : NSFilePromiseProvider {
  NSArray *delegateTypes;
}

@property(strong, nullable) id<NSPasteboardWriting> writingDelegate;

@end

@implementation SNEForwardingFilePromiseProvider

- (NSArray<NSPasteboardType> *)writableTypesForPasteboard:
    (NSPasteboard *)pasteboard {
  delegateTypes = [self.writingDelegate writableTypesForPasteboard:pasteboard];
  NSMutableArray *types = [NSMutableArray
      arrayWithArray:[super writableTypesForPasteboard:pasteboard]];
  [types addObjectsFromArray:delegateTypes];
  return types;
}

- (NSPasteboardWritingOptions)writingOptionsForType:(NSPasteboardType)type
                                         pasteboard:(NSPasteboard *)pasteboard;
{
  if ([delegateTypes containsObject:type]) {
    return [self.writingDelegate writingOptionsForType:type
                                            pasteboard:pasteboard];
  } else {
    return [super writingOptionsForType:type pasteboard:pasteboard];
  }
}

- (nullable id)pasteboardPropertyListForType:(NSPasteboardType)type {
  if ([delegateTypes containsObject:type]) {
    return [self.writingDelegate pasteboardPropertyListForType:type];
  } else {
    return [super pasteboardPropertyListForType:type];
  }
}

@end