import 'dart:async';

import 'encoded_data.dart';
import 'platform.dart';
import 'writer.dart';

/// Callback to obtain data lazily. See [EncodableDataFormat.encodeLazy];
typedef DataProvider<T> = FutureOr<T> Function();

/// Platform specific name for data format. For example value for plain text
/// for macOS and iOS would be 'public.utf8-plain-text', for android, Linux
/// and web 'text/plain' and for windows 'NativeShell_InternalWindowsFormat_13',
/// which maps to CF_UNICODETEXT (value of 13).
typedef PlatformFormat = String;

/// Base class for platform independent data format.
abstract class DataFormat {
  const DataFormat();

  /// Returns name of primary platform format for this data format.
  /// This format will be used when writing / reading virtual files.
  PlatformFormat get primaryFormat =>
      primaryFormatForPlatform(_currentPlatform);

  ClipboardPlatform get _currentPlatform => clipboardPlatform;

  PlatformFormat primaryFormatForPlatform(ClipboardPlatform platform);
}

/// Codec for encoding and decoding data from/to platform specific format.
abstract class PlatformCodec<T> {
  List<PlatformFormat> get encodableFormats;
  FutureOr<Object> encode(T t, PlatformFormat format);

  List<PlatformFormat> get decodableFormats;
  FutureOr<T> decode(Object data, PlatformFormat format);
}

/// Data format that supports encoding / decoding values. Used when writing to
/// and reading from clipboard.
abstract class EncodableDataFormat<T> extends DataFormat {
  const EncodableDataFormat();

  /// Encodes the provided data to platform specific format. This encoded data
  /// can be used to initialize [DataWriterItem].
  FutureOr<EncodedData> encode(T data) async {
    final encoder = codecForPlatform(_currentPlatform);
    final entries = <EncodedDataEntry>[];
    for (final format in encoder.encodableFormats) {
      entries.add(
        EncodedDataEntrySimple(format, await encoder.encode(data, format)),
      );
    }
    return EncodedData(entries);
  }

  /// Encodes the provided lazy data. Some platforms support providing the data
  /// on demand. In which case the DataProvider callback will be invoked when
  /// the data is requested. On platforms that do not support this (iOS, web)
  /// the provider callback will be called eagerly.
  FutureOr<EncodedData> encodeLazy(DataProvider<T> provider) {
    final encoder = codecForPlatform(_currentPlatform);
    final entries = <EncodedDataEntry>[];
    for (final format in encoder.encodableFormats) {
      entries.add(
        EncodedDataEntryLazy(
            format, () async => encoder.encode(await provider(), format)),
      );
    }
    return EncodedData(entries);
  }

  bool canDecode(PlatformFormat format) {
    return decodableFormats.contains(format);
  }

  List<String> get decodableFormats =>
      codecForPlatform(_currentPlatform).decodableFormats;

  FutureOr<T> decode(PlatformFormat format, Object data) {
    return codecForPlatform(_currentPlatform).decode(data, format);
  }

  @override
  String primaryFormatForPlatform(ClipboardPlatform platform) {
    return codecForPlatform(platform).encodableFormats.first;
  }

  PlatformCodec<T> codecForPlatform(ClipboardPlatform platform);
}
