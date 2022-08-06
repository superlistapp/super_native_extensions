import 'package:flutter/widgets.dart';

import '../api_model.dart';
import '../data_provider.dart';
import '../drag.dart';
import '../drop.dart';
import '../reader.dart';
import '../reader_manager.dart';

import 'reader_manager.dart';

class DropContextImpl extends DropContext {
  static DropContextImpl? instance;

  DropContextImpl() {
    instance = this;
  }

  @override
  Future<void> initialize() async {}

  @override
  Future<void> registerDropFormats(List<String> formats) async {}

  DropEvent _createLocalDropEvent({
    required DragConfiguration configuration,
    required Offset position,
    DropOperation? acceptedOperation,
  }) {
    List<String> itemFormats(DataProviderHandle handle) {
      return handle.provider.representations.map((e) => e.format).toList();
    }

    return DropEvent(
      sessionId: identityHashCode(configuration),
      locationInView: position,
      allowedOperations: configuration.allowedOperations,
      items: configuration.items
          .map(
            (item) => DropItem(
              itemId: identityHashCode(item),
              formats: itemFormats(item.dataProvider),
              localData: item.localData,
              readerItem: DataReaderItem(
                handle: DataProviderReaderItem(item.dataProvider)
                    as DataReaderItemHandle,
              ),
            ),
          )
          .toList(growable: false),
      acceptedOperation: acceptedOperation,
    );
  }

  Future<DropOperation> localSessionDidMove(
    DragConfiguration configuration,
    Offset position,
  ) async {
    return await delegate?.onDropUpdate(
          _createLocalDropEvent(
            configuration: configuration,
            position: position,
          ),
        ) ??
        DropOperation.none;
  }

  Future<void> localSessionDrop(
    DragConfiguration configuration,
    Offset position,
    DropOperation acceptedOperation,
  ) async {
    await delegate?.onPerformDrop(
      _createLocalDropEvent(
        configuration: configuration,
        position: position,
        acceptedOperation: acceptedOperation,
      ),
    );
  }

  void localSessionDidEnd(DragConfiguration configuration) {
    final event = BaseDropEvent(sessionId: identityHashCode(configuration));
    delegate?.onDropLeave(event);
    delegate?.onDropEnded(event);
  }
}
