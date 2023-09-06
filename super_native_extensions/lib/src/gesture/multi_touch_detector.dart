import 'package:flutter/widgets.dart';

final _activePointers = <int>{};

class MultiTouchDetector extends StatelessWidget {
  const MultiTouchDetector({
    super.key,
    required this.child,
  });

  final Widget child;

  static bool isMultiTouchActive() {
    return _activePointers.length > 1;
  }

  @override
  Widget build(BuildContext context) {
    return Listener(
      child: child,
      onPointerDown: (event) {
        _activePointers.add(event.pointer);
      },
      onPointerUp: (event) {
        _activePointers.remove(event.pointer);
      },
      onPointerCancel: (event) {
        _activePointers.remove(event.pointer);
      },
    );
  }
}
