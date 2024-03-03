import 'dart:js_interop';
import 'dart:ui' as ui;

import 'package:web/web.dart' as web;
import '../drag_interaction/long_press_session.dart';
import '../menu_flutter.dart';

class MenuContextImpl extends FlutterMenuContext {
  @override
  Future<void> initialize() async {
    await super.initialize();

    web.document.addEventListener(
      'contextmenu',
      (web.MouseEvent event) {
        final offset = ui.Offset(
          event.clientX.toDouble(),
          event.clientY.toDouble(),
        );
        final contextMenuAllowed =
            delegate?.contextMenuIsAllowed(offset) ?? false;
        if (contextMenuAllowed) {
          event.preventDefault();
        }
        if (LongPressSession.active) {
          event.preventDefault();
        }
      }.toJS,
    );
  }
}
