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

@interface SNEDeletingPresenter : NSObject <NSFilePresenter> {
  NSURL *url;
  NSOperationQueue *queue;
}

- (instancetype)initWithURL:(NSURL *)url;

@end

@implementation SNEDeletingPresenter

+ (void)deleteAfterRead:(NSURL *)url {
  SNEDeletingPresenter *presenter =
      [[SNEDeletingPresenter alloc] initWithURL:url];
  [NSFileCoordinator addFilePresenter:presenter];
}

- (instancetype)initWithURL:(NSURL *)url {
  if (self = [super init]) {
    self->url = url;
    self->queue = [NSOperationQueue new];
  }
  return self;
}

- (NSURL *)presentedItemURL {
  return self->url;
}

- (NSOperationQueue *)presentedItemOperationQueue {
  return self->queue;
}

- (void)relinquishPresentedItemToReader:
    (void (^)(void (^_Nullable)(void)))reader {
  reader(^{
    NSError *error;
    [[NSFileManager defaultManager] removeItemAtURL:self->url error:&error];
    if (error != nil) {
      NSLog(@"Error deleting file %@", error);
    }
    [NSFileCoordinator removeFilePresenter:self];
  });
}

@end
