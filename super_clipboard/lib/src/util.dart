import 'package:flutter/foundation.dart';

class SimpleNotifier extends ChangeNotifier {
  void notify() {
    super.notifyListeners();
  }
}

class Pair<T, U> {
  const Pair(this.first, this.second);

  final T first;
  final U second;
}
