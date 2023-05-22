import 'dart:html' as html;
import 'dart:ui' as ui;

import '../drag_interaction/long_press_session.dart';
import '../menu_flutter.dart';

class MenuContextImpl extends FlutterMenuContext {
  @override
  Future<void> initialize() async {
    await super.initialize();

    html.document.addEventListener('contextmenu', (event) {
      final offset_ = (event as html.MouseEvent).client;
      final offset = ui.Offset(offset_.x.toDouble(), offset_.y.toDouble());
      final contextMenuAllowed =
          delegate?.contextMenuIsAllowed(offset) ?? false;
      if (contextMenuAllowed) {
        event.preventDefault();
      }
      if (LongPressSession.active) {
        event.preventDefault();
      }
    });
  }
}
