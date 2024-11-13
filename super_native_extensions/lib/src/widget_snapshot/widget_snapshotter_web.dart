import 'dart:js_interop' as js;
import 'dart:js_interop_unsafe';

import 'package:flutter/foundation.dart';

final isCanvasKit = js.globalContext['flutterCanvasKit'] != null;

bool snapshotToImageSupportedInternal() {
  return isCanvasKit || kIsWasm;
}
