import 'dart:async';
import 'dart:typed_data';

import 'package:meta/meta.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;

import 'reader.dart';
import 'writer.dart';

/// Callback to obtain data lazily. See [DataFormat.lazy];
typedef DataProvider<T> = FutureOr<T> Function();

/// Platform specific name for data format. For example value for plain text
/// for macOS and iOS would be 'public.utf8-plain-text', for android, Linux
/// and web 'text/plain' and for windows 'NativeShell_CF_13',  which maps
/// to CF_UNICODETEXT (value of 13).
typedef PlatformFormat = String;

abstract class PlatformDataProvider {
  /// Returns data for the given [format] if available.
  /// Note that decoder must request data for all formats it needs before
  /// crossing async boundary.
  Future<Object?> getData(PlatformFormat format);

  /// Returns all formats available in this provider.
  List<PlatformFormat> getAllFormats();
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

  /// Decodes the data from platform representation.
  /// Returns `null` if decoding failed.
  ///
  /// Important: When implementing custom decoder it is necessary to request
  /// all data from dataProvider before awaiting on anything.
  //
  /// Default implementation simply attempts to cast to target format.
  Future<T?> decode(
      PlatformDataProvider dataProvider, PlatformFormat format) async {
    final value = await dataProvider.getData(format);
    return value is T ? value : null;
  }
}

/// Base class for formats of data transfered to clipboard and drag & drop.
/// This branches into [ValueFormat] for data values that need to be converted
/// from and to platform specific formats (such as plain text, HTML snippet,
/// uri) and [FileFormat] representing files that are processed without
/// conversion (i.e. PNG, JPEG).
@sealed
abstract class DataFormat<T extends Object> {
  const DataFormat();

  /// Encodes the provided data to platform specific format.
  /// The encoded data can be added to [DataWriterItem].
  ///
  /// ```dart
  /// final item = DataWriterItem();
  /// item.add(Format.plainText('Hello World'));
  /// ```
  FutureOr<EncodedData> call(T data);

  /// Encodes the provided lazy data. Some platforms support providing the data
  /// on demand. In which case the [provider] callback will be invoked when
  /// the data is requested. On platforms that do not support this (iOS, web)
  /// the [provider] callback will be called eagerly.
  FutureOr<EncodedData> lazy(DataProvider<T> provider);
}

/// Format for values that need to be converted from and to platform specific
/// formats (such as plain text, HTML snippet, uri).
///
/// These formats can be used to provide and receive values, but not for
/// generating and receiving virtual files.
abstract class ValueFormat<T extends Object> extends DataFormat<T> {
  const ValueFormat();

  PlatformCodec<T> get codec;

  @override
  FutureOr<EncodedData> call(T data) async {
    final encoder = codec;
    final entries = <raw.DataRepresentation>[];
    for (final format in encoder.encodingFormats) {
      entries.add(
        raw.DataRepresentation.simple(
            format: format, data: await encoder.encode(data, format)),
      );
    }
    return EncodedData(entries);
  }

  @override
  FutureOr<EncodedData> lazy(DataProvider<T> provider) {
    final encoder = codec;
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

  @override
  String toString() {
    return '$runtimeType (${codec.decodingFormats.first})';
  }
}

/// Base format class for files that are in standardized formats and processed
/// without conversion (i.e. PNG, JPEG).
///
/// These format can be used to provide and receive values, but also for
/// providing and receiving virtual files (i.e. content generated on demand).
abstract class FileFormat extends DataFormat<Uint8List> {
  const FileFormat();

  /// Platform format used when providing the virtual file.
  PlatformFormat get providerFormat;

  /// List of platform formats used when obtaining [VirtualFileReceiver]
  /// from [DataReader].
  /// First formats for the list that yields a receiver will be used.
  List<PlatformFormat> get receiverFormats;

  @override
  FutureOr<EncodedData> call(Uint8List data) {
    return EncodedData([
      raw.DataRepresentation.simple(
        format: providerFormat,
        data: data,
      ),
    ]);
  }

  @override
  FutureOr<EncodedData> lazy(DataProvider<Uint8List> provider) {
    return EncodedData([
      raw.DataRepresentation.lazy(
        format: providerFormat,
        dataProvider: provider,
      ),
    ]);
  }

  @override
  String toString() {
    return '$runtimeType ($providerFormat)';
  }
}
