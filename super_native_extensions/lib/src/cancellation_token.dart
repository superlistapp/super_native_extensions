import 'package:flutter/foundation.dart';

abstract class CancellationToken implements Listenable {
  bool get cancelled;
}

class SimpleCancellationToken extends CancellationToken {
  @override
  void addListener(VoidCallback listener) {
    _notifier.addListener(listener);
  }

  @override
  bool get cancelled => _notifier.value;

  @override
  void removeListener(VoidCallback listener) {
    _notifier.removeListener(listener);
  }

  void cancel() {
    if (!_disposed) {
      _notifier.value = true;
      dispose();
    }
  }

  void dispose() {
    _notifier.dispose();
    _disposed = true;
  }

  bool _disposed = false;

  final _notifier = ValueNotifier(false);
}
