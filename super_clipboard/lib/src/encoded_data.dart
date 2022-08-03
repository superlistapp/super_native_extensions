import 'format.dart';

/// Clipboard data in platform specific format. Do not use directly.
class EncodedData {
  EncodedData(this.entries);

  final List<EncodedDataEntry> entries;
}

abstract class EncodedDataEntry {
  EncodedDataEntry(this.format);

  final PlatformFormat format;
}

class EncodedDataEntrySimple extends EncodedDataEntry {
  EncodedDataEntrySimple(super.format, this.data);

  final Object data;
}

class EncodedDataEntryLazy extends EncodedDataEntry {
  EncodedDataEntryLazy(super.format, this.dataProvider);

  final DataProvider<Object> dataProvider;
}
