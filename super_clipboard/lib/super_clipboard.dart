library super_clipboard;

import 'package:super_data_transfer/super_data_transfer.dart';

/// A Calculator.
class Calculator {
  /// Returns [value] plus 1.
  int addOne(int value) => value + 1;
}

void testReader() async {
  RawClipboardReader reader = await RawClipboardReader.newDefaultReader();
  await reader.getItems();
}
