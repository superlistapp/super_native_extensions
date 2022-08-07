import 'package:flutter/foundation.dart';

class SimpleNotifier extends ChangeNotifier {
  void notify() {
    super.notifyListeners();
  }
}
