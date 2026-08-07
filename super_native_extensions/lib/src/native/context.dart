import 'dart:ffi';
import 'dart:io' as io;

import 'package:flutter/foundation.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

MessageChannelContext _getNativeContext() {
  if (io.Platform.environment.containsKey('FLUTTER_TEST')) {
    // FFI doesn't work in Flutter Tester
    return MockMessageChannelContext();
  } else {
    final dylib = openNativeLibrary();
    final function = dylib
        .lookup<NativeFunction<MessageChannelContextInitFunction>>(
          "super_native_extensions_init_message_channel_context",
        );
    return MessageChannelContext.forInitFunction(function);
  }
}

final _nativeContext = _getNativeContext();

MessageChannelContext? _contextOverride;

@visibleForTesting
void setContextOverride(MessageChannelContext context) {
  _contextOverride = context;
}

MessageChannelContext get superNativeExtensionsContext =>
    _contextOverride ?? _nativeContext;

DynamicLibrary openNativeLibrary() {
  const libraryName = 'super_native_extensions_native';
  if (io.Platform.isIOS || io.Platform.isMacOS) {
    return DynamicLibrary.open('@rpath/$libraryName.framework/$libraryName');
  }
  if (io.Platform.isAndroid || io.Platform.isLinux) {
    return DynamicLibrary.open('lib$libraryName.so');
  }
  if (io.Platform.isWindows) {
    return DynamicLibrary.open('$libraryName.dll');
  }
  throw UnsupportedError('Unsupported platform ${io.Platform.operatingSystem}');
}
