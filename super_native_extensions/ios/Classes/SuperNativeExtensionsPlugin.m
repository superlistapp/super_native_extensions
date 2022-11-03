#import "SuperNativeExtensionsPlugin.h"

extern void super_native_extensions_init(void);

@implementation SuperNativeExtensionsPlugin

+ (void)initialize {
  super_native_extensions_init();
}

+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {
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
