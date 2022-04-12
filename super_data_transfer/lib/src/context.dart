import 'dart:ffi';

import 'package:flutter/foundation.dart';
import 'package:nativeshell_core/nativeshell_core.dart';

MessageChannelContext _getContext() {
  final dylib = defaultTargetPlatform == TargetPlatform.android
      ? DynamicLibrary.open("libsuper_data_transfer.so")
      : (defaultTargetPlatform == TargetPlatform.windows
          ? DynamicLibrary.open("super_data_transfer.dll")
          : DynamicLibrary.process());
  final function =
      dylib.lookup<Void>("super_data_transfer_init_message_channel_context");
  return MessageChannelContext.forInitFunction(function);
}

final MessageChannelContext superDataTransferContext = _getContext();
