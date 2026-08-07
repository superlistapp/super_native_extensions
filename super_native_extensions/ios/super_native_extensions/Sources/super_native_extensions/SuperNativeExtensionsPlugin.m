#import "./include/super_native_extensions/SuperNativeExtensionsPlugin.h"

#include <objc/runtime.h>
#include <stdbool.h>
#include <dlfcn.h>

typedef void (*SNEInitFunction)(void);
typedef bool (*SNETextInputFunction)(void);

static void swizzleTextInputPlugin();

static void *SNEOpenRustLibrary() {
  static void *handle = NULL;
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    handle = dlopen("@rpath/super_native_extensions_native.framework/"
                    "super_native_extensions_native",
                    RTLD_NOW | RTLD_LOCAL);
    if (handle == NULL) {
      NSLog(@"Failed to load super_native_extensions_native.framework: %s",
            dlerror());
    }
  });
  return handle;
}

static void *SNERustSymbol(const char *name) {
  void *handle = SNEOpenRustLibrary();
  if (handle == NULL) {
    return NULL;
  }

  dlerror();
  void *symbol = dlsym(handle, name);
  const char *error = dlerror();
  if (error != NULL) {
    NSLog(@"Failed to resolve %s: %s", name, error);
    return NULL;
  }
  return symbol;
}

static void SNEInitializeRustLibrary() {
  static dispatch_once_t onceToken;
  dispatch_once(&onceToken, ^{
    SNEInitFunction init =
        (SNEInitFunction)SNERustSymbol("super_native_extensions_init");
    if (init != NULL) {
      init();
    }
  });
}

static bool SNECallTextInputFunction(const char *name) {
  SNETextInputFunction function = (SNETextInputFunction)SNERustSymbol(name);
  return function != NULL && function();
}

@implementation SuperNativeExtensionsPlugin

+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {
  SNEInitializeRustLibrary();
  swizzleTextInputPlugin();
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
  if (!SNECallTextInputFunction("super_native_extensions_text_input_plugin_cut")) {
    [self cut_:sender];
  }
}

- (void)copy_:(id)sender {
  if (!SNECallTextInputFunction(
          "super_native_extensions_text_input_plugin_copy")) {
    [self copy_:sender];
  }
}

- (void)paste_:(id)sender {
  if (!SNECallTextInputFunction(
          "super_native_extensions_text_input_plugin_paste")) {
    [self paste_:sender];
  }
}

- (void)selectAll_:(id)sender {
  if (!SNECallTextInputFunction(
          "super_native_extensions_text_input_plugin_select_all")) {
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
