import 'dart:js' as js;

final isCanvasKit = js.context['flutterCanvasKit'] != null;

bool snapshotToImageSupportedInternal() {
  return isCanvasKit;
}
