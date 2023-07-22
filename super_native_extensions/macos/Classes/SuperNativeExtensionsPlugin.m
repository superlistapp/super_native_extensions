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

@interface SNEBlockMenuItem : NSMenuItem

@property(nonatomic, readwrite, copy) void (^handler)(NSMenuItem *item);

- (id)initWithTitle:(NSString *)title
      keyEquivalent:(NSString *)keyEquivalent
              block:(void (^)(NSMenuItem *item))block;

@end

@implementation SNEBlockMenuItem

- (id)initWithTitle:(NSString *)title
      keyEquivalent:(NSString *)keyEquivalent
              block:(void (^)(NSMenuItem *item))block {
  if (self = [super initWithTitle:title
                           action:block != nil ? @selector(_onAction:) : nil
                    keyEquivalent:keyEquivalent]) {
    self.target = self;
    self.handler = block;
  }
  return self;
}

- (void)_onAction:(NSMenuItem *)item {
  self.handler(item);
}

@end

@interface SNEDeferredMenuItem : NSMenuItem

@property(nonatomic, readwrite, copy) void (^loader_block)(NSMenuItem *item);

- (id)initWithBlock:(void (^)(NSMenuItem *item))block;

@end

@interface SNEMenuContainerView : NSView

@property(readwrite, nonatomic, weak) NSMenuItem *containerItem;

@end

@implementation SNEMenuContainerView

- (void)layout {
  BOOL hasState = NO;
  for (NSMenuItem *item in self.containerItem.menu.itemArray) {
    if (item.state != NSControlStateValueOff) {
      hasState = YES;
      break;
    }
  }

  NSView *subview = self.subviews[0];
  NSRect frame = subview.frame;
  if (hasState) {
    frame.origin.x = 20;
  } else {
    frame.origin.x = 10;
  }
  subview.frame = frame;

  [super layout];
}

@end

@implementation SNEDeferredMenuItem

- (id)initWithBlock:(void (^)(NSMenuItem *item))block {
  if (self = [super initWithTitle:@"" action:nil keyEquivalent:@""]) {
    SNEMenuContainerView *view =
        [[SNEMenuContainerView alloc] initWithFrame:NSMakeRect(0, 0, 50, 22)];

    view.containerItem = self;

    view.autoresizingMask = NSViewWidthSizable;

    NSProgressIndicator *indicator = [[NSProgressIndicator alloc] init];
    indicator.style = NSProgressIndicatorStyleSpinning;
    indicator.controlSize = NSControlSizeSmall;
    indicator.frame = NSMakeRect(0, 4, 22, 14);
    [indicator startAnimation:nil];

    [view addSubview:indicator];

    self.view = view;
    self.loader_block = block;
  }
  return self;
}

@end

@interface SNEMenu : NSMenu <NSMenuDelegate> {
  BOOL _didLoad;
}

- (id)initWithTitle:(NSString *)title;

@end

@implementation SNEMenu

- (id)initWithTitle:(NSString *)title {
  if (self = [super initWithTitle:title]) {
    self.delegate = self;
  }
  return self;
}

- (void)menuWillOpen:(NSMenu *)menu {
  if (!_didLoad) {
    _didLoad = YES;
    for (NSMenuItem *item in self.itemArray) {
      if ([item isKindOfClass:[SNEDeferredMenuItem class]]) {
        SNEDeferredMenuItem *deferredItem = (SNEDeferredMenuItem *)item;
        deferredItem.loader_block(deferredItem);
      }
    }
  }
}

@end
