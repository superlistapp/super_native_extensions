#import "SuperNativeExtensionsPlugin.h"

extern void super_native_extensions_init(void);

@implementation SuperNativeExtensionsPlugin

+ (void)initialize {
  super_native_extensions_init();
}

+ (void)registerWithRegistrar:(NSObject<FlutterPluginRegistrar> *)registrar {
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
