import 'dart:async';

import 'package:super_native_extensions/raw_clipboard.dart' as raw;
import 'package:collection/collection.dart';

import 'platform.dart';
import 'reader.dart';
import 'writer.dart';

/// Callback to obtain data lazily. See [EncodableDataFormat.encodeLazy];
typedef DataProvider<T> = FutureOr<T> Function();

/// Platform specific name for data format. For example value for plain text
/// for macOS and iOS would be 'public.utf8-plain-text', for android, Linux
/// and web 'text/plain' and for windows 'NativeShell_InternalWindowsFormat_13',
/// which maps to CF_UNICODETEXT (value of 13).
typedef PlatformFormat = String;

typedef PlatformDataProvider = Future<Object?> Function(PlatformFormat);

/// Format for a virtual file. Provides platform formats for providing
/// and receiving virtual files. However unlike [DataFormat] there is no
/// codec as the files are received without modifications.
///
/// For convenience any [DataFormat] is also a [VirtualFileFormat], though
/// the codec is only used to provide platform formats and not to encode
/// and decode data.
abstract class VirtualFileFormat {
  const VirtualFileFormat();

  /// Platform format used when providing the virtual file.
  PlatformFormat? get providerFormat;

  /// List of platform formats used when obtaining [VirtualFileReceiver]
  /// from [DataReader].
  /// First formats for the list that yields a receiver will be used.
  List<PlatformFormat> get receiverFormats;
}

/// DataFormat encapsulates [PlatformFormat]s for specific data type
/// as well as logic to encode and decode data to platform specific formats.
abstract class DataFormat<T extends Object> extends VirtualFileFormat {
  const DataFormat();

  PlatformCodec<T> codecForPlatform(ClipboardPlatform platform);

  /// Encodes the provided data to platform specific format.
  /// The encoded data can be added to [DataWriterItem].
  FutureOr<EncodedData> encode(T data) async {
    final encoder = codecForPlatform(currentPlatform);
    final entries = <raw.DataRepresentation>[];
    for (final format in encoder.encodingFormats) {
      entries.add(
        raw.DataRepresentation.simple(
            format: format, data: await encoder.encode(data, format)),
      );
    }
    return EncodedData(entries);
  }

  /// Encodes the provided lazy data. Some platforms support providing the data
  /// on demand. In which case the DataProvider callback will be invoked when
  /// the data is requested. On platforms that do not support this (iOS, web)
  /// the provider callback will be called eagerly.
  FutureOr<EncodedData> encodeLazy(DataProvider<T> provider) {
    final encoder = codecForPlatform(currentPlatform);
    final entries = <raw.DataRepresentation>[];
    for (final format in encoder.encodingFormats) {
      entries.add(
        raw.DataRepresentation.lazy(
            format: format,
            dataProvider: () async => encoder.encode(await provider(), format)),
      );
    }
    return EncodedData(entries);
  }

  bool canDecode(PlatformFormat format) {
    final decoder = codecForPlatform(currentPlatform);
    return decoder.decodingFormats.contains(format);
  }

  FutureOr<T?> decode(PlatformFormat format, PlatformDataProvider provider) {
    final decoder = codecForPlatform(currentPlatform);
    return decoder.decode(provider, format);
  }

  @override
  List<PlatformFormat> get receiverFormats =>
      codecForPlatform(currentPlatform).decodingFormats;

  @override
  PlatformFormat? get providerFormat =>
      codecForPlatform(currentPlatform).encodingFormats.firstOrNull;
}

/// Clipboard data in platform specific format. Do not use directly.
class EncodedData {
  EncodedData(this.representations);

  final List<raw.DataRepresentation> representations;
}

/// Platform specific codec for a data format.
abstract class PlatformCodec<T extends Object> {
  const PlatformCodec();

  List<PlatformFormat> get encodingFormats;

  /// Encodes the data to platform representation. By default this
  /// is a simple passthrough function.
  FutureOr<Object?> encode(T value, PlatformFormat format) {
    return value;
  }

  List<PlatformFormat> get decodingFormats;

  List<PlatformFormat> get receiverFormats => decodingFormats;

  /// Decodes the data from platform representation.
  /// Returns `null` if decoding failed.
  //
  /// Default implementation simply attempts to tast to target format.
  FutureOr<T?> decode(
      PlatformDataProvider dataProvider, PlatformFormat format) async {
    final value = await dataProvider(format);
    return value is T ? value : null;
  }
}
