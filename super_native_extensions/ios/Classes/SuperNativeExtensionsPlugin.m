#import "SuperNativeExtensionsPlugin.h"

#include <objc/runtime.h>

extern void super_native_extensions_init(void);
extern bool super_native_extensions_text_input_plugin_cut(void);
extern bool super_native_extensions_text_input_plugin_copy(void);
extern bool super_native_extensions_text_input_plugin_paste(void);
extern bool super_native_extensions_text_input_plugin_select_all(void);

static void swizzleTextInputPlugin();

@implementation SuperNativeExtensionsPlugin

+ (void)initialize {
  super_native_extensions_init();
  swizzleTextInputPlugin();
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

@interface SNETextInputPlugin : NSObject
@end

@implementation SNETextInputPlugin

- (void)cut_:(id)sender {
  if (!super_native_extensions_text_input_plugin_cut()) {
    [self cut_:sender];
  }
}

- (void)copy_:(id)sender {
  if (!super_native_extensions_text_input_plugin_copy()) {
    [self copy_:sender];
  }
}

- (void)paste_:(id)sender {
  if (!super_native_extensions_text_input_plugin_paste()) {
    [self paste_:sender];
  }
}

- (void)selectAll_:(id)sender {
  if (!super_native_extensions_text_input_plugin_select_all()) {
    [self selectAll_:sender];
  }
}

@end

static void swizzle(SEL originalSelector, Class originalClass,
                    SEL replacementSelector, Class replacementClass) {
  Method origMethod = class_getInstanceMethod(originalClass, originalSelector);

  if (!origMethod) {
#if DEBUG
    NSLog(@"Original method %@ not found for class %s",
          NSStringFromSelector(originalSelector), class_getName(originalClass));
#endif
    return;
  }

  Method altMethod =
      class_getInstanceMethod(replacementClass, replacementSelector);
  if (!altMethod) {
#if DEBUG
    NSLog(@"Alternate method %@ not found for class %s",
          NSStringFromSelector(replacementSelector),
          class_getName(originalClass));
#endif
    return;
  }

  class_addMethod(
      originalClass, originalSelector,
      class_getMethodImplementation(originalClass, originalSelector),
      method_getTypeEncoding(origMethod));
  class_addMethod(
      originalClass, replacementSelector,
      class_getMethodImplementation(replacementClass, replacementSelector),
      method_getTypeEncoding(altMethod));

  method_exchangeImplementations(
      class_getInstanceMethod(originalClass, originalSelector),
      class_getInstanceMethod(originalClass, replacementSelector));
}

static void swizzleTextInputPlugin() {
  Class cls = NSClassFromString(@"FlutterTextInputView");
  if (cls == nil) {
    NSLog(@"FlutterTextInputPlugin not found");
    return;
  }

  Class replacement = [SNETextInputPlugin class];
  swizzle(@selector(cut:), cls, @selector(cut_:), replacement);
  swizzle(@selector(copy:), cls, @selector(copy_:), replacement);
  swizzle(@selector(paste:), cls, @selector(paste_:), replacement);
  swizzle(@selector(selectAll:), cls, @selector(selectAll_:), replacement);
}
