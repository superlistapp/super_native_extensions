import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:super_native_extensions/raw_clipboard.dart' as raw;
export 'package:super_native_extensions/raw_clipboard.dart'
    show
        VirtualFileProvider,
        VirtualFileEventSinkProvider,
        WriteProgress,
        VirtualFileStorage;

import 'format.dart';
import 'util.dart';
import 'writer.dart';
import 'writer_data_provider.dart';

/// Represents a single item in the clipboard. The item can have multiple
/// renditions (each represented as entries in [EncodedData]).
/// To get encoded data for values use [DataFormat.call] or
/// [DataFormat.lazy];
class DataWriterItem {
  DataWriterItem({this.suggestedName});

  /// Adds representation to the data item. On item can contain multiple
  /// representations, each in a different format. Representation should
  /// be added by priority (highest fidelity content first), as some
  /// platforms respect the order.
  void add(FutureOr<EncodedData> data) {
    _data.add(data);
  }

  bool get virtualFileSupported =>
      !kIsWeb &&
      (defaultTargetPlatform == TargetPlatform.windows ||
          defaultTargetPlatform == TargetPlatform.iOS);

  /// Adds a virtual file to this data item. Virtual files are files generated
  /// on demand, possibly taking long time to complete (i.e. downloading from
  /// internet).
  ///
  /// Only one virtual file per data item is supported. For clipboard, virtual
  /// files are supported on iOS and Windows. For Drag & Drop, virtual files are
  /// also supported on macOS. You can use [virtualFileSupported] to check
  /// whether current platform supports virtual files.
  void addVirtualFile({
    required FileFormat format,
    required VirtualFileProvider provider,
    VirtualFileStorage? storageSuggestion,
  }) {
    assert(virtualFileSupported);
    _data.add(EncodedData([
      raw.DataRepresentation.virtualFile(
        format: format.providerFormat,
        virtualFileProvider: provider,
        storageSuggestion: storageSuggestion,
      )
    ]));
  }

  /// Invoked when the item is successfully registered with native code.
  Listenable get onRegistered => _onRegistered;

  /// Called when the native code is done with the item and the data is
  /// no longer needed. Only guaranteed to be called if [onRegistered] has
  /// been called before.
  Listenable get onDisposed => _onDisposed;

  final _onRegistered = SimpleNotifier();
  final _onDisposed = SimpleNotifier();
  final _data = <FutureOr<EncodedData>>[];

  /// File name suggestion for the client receiving this data item.
  final String? suggestedName;

  List<FutureOr<EncodedData>> get data => _data;
}

/// Example for using clipboard writer:
/// ```dart
/// final item = ClipboardWriterItem();
/// item.addData(formatHtmlText.encode('<b><i>Html</i></b> Value'));
/// item.addData(formatPlainText.encodeLazy(() =>
///                                   'Plaintext value resolved lazily'));
/// await ClipboardWriter.instance.write([item]);
/// ```
abstract class ClipboardWriter {
  /// Writes the provided items in system clipboard.
  Future<void> write(Iterable<DataWriterItem> items);

  static final instance = _ClipboardWriter();
}

class _ClipboardWriter extends ClipboardWriter {
  @override
  Future<void> write(Iterable<DataWriterItem> items) async {
    await items.withHandles((handles) async {
      await raw.ClipboardWriter.instance.write(handles);
    });
  }
}
