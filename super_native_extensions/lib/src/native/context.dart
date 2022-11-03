import 'dart:ffi';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

MessageChannelContext _getNativeContext() {
  final dylib = openNativeLibrary();
  final function =
      dylib.lookup<NativeFunction<MessageChannelContextInitFunction>>(
          "super_native_extensions_init_message_channel_context");
  return MessageChannelContext.forInitFunction(function);
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
  final dylib = defaultTargetPlatform == TargetPlatform.android
      ? DynamicLibrary.open("libsuper_native_extensions.so")
      : (defaultTargetPlatform == TargetPlatform.windows
          ? DynamicLibrary.open("super_native_extensions.dll")
          : DynamicLibrary.process());
  return dylib;
}
