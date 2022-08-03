import 'dart:ui';

import 'package:super_native_extensions/raw_drag_drop.dart';

class DragContextImpl extends DragContext {
  @override
  Future<void> initialize() async {}

  @override
  DragSession newSession() {
    throw UnimplementedError();
  }

  @override
  Future<DragSession> startDrag({
    required DragSession session,
    required DragConfiguration configuration,
    required Offset position,
  }) {
    // TODO: implement startDrag
    throw UnimplementedError();
  }
}
