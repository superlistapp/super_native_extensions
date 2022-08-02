import 'dart:async';

import 'encoded_data.dart';
import 'platform.dart';

typedef DataProvider<T> = FutureOr<T> Function();

abstract class PlatformEncoder<T> {
  List<String> get formats;
  FutureOr<Object> encode(T t, String format);
}

abstract class PlatformDecoder<T> {
  bool canDecode(String format);
  FutureOr<T> decode(Object data, String format);
}

abstract class DataFormat<T> {
  const DataFormat();

  FutureOr<EncodedData> encode(T data) async {
    final encoder = encoderForPlatform(_currentPlatform);
    final entries = <EncodedDataEntry>[];
    for (final format in encoder.formats) {
      entries.add(
        EncodedDataEntrySimple(format, await encoder.encode(data, format)),
      );
    }
    return EncodedData(entries);
  }

  FutureOr<EncodedData> encodeLazy(DataProvider<T> provider) {
    final encoder = encoderForPlatform(_currentPlatform);
    final entries = <EncodedDataEntry>[];
    for (final format in encoder.formats) {
      entries.add(
        EncodedDataEntryLazy(
            format, () async => encoder.encode(await provider(), format)),
      );
    }
    return EncodedData(entries);
  }

  bool canHandle(String format) {
    return decoderForPlatform(_currentPlatform).canDecode(format);
  }

  FutureOr<T> decode(String format, Object data) {
    return decoderForPlatform(_currentPlatform).decode(data, format);
  }

  ClipboardPlatform get _currentPlatform => clipboardPlatform;

  PlatformEncoder<T> encoderForPlatform(ClipboardPlatform platform);
  PlatformDecoder<T> decoderForPlatform(ClipboardPlatform platform);
}
