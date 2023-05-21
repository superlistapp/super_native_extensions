import 'dart:ffi';
import 'dart:io' show Platform;

import 'package:flutter/foundation.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

MessageChannelContext _getNativeContext() {
  if (Platform.environment.containsKey('FLUTTER_TEST')) {
    // FFI doesn't work in Flutter Tester
    return MockMessageChannelContext();
  } else {
    final dylib = openNativeLibrary();
    final function =
        dylib.lookup<NativeFunction<MessageChannelContextInitFunction>>(
            "super_native_extensions_init_message_channel_context");
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
  final dylib = Platform.isAndroid
      ? DynamicLibrary.open("libsuper_native_extensions.so")
      : (Platform.isWindows
          ? DynamicLibrary.open("super_native_extensions.dll")
          : DynamicLibrary.process());
  return dylib;
}
