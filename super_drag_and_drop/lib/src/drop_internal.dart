import 'dart:async';
import 'dart:ui' as ui;
import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

// ignore: implementation_imports, // Needed for FormatExtensions
import 'package:super_clipboard/src/reader_internal.dart';

import 'util.dart';
import 'drop.dart';

class _DropItem extends DropItem {
  _DropItem._(this._item);

  @override
  bool hasValue(DataFormat f) {
    return _item.formats.any(f.canDecode);
  }

  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(DiagnosticsProperty('formats', _item.formats));
    properties.add(DiagnosticsProperty('localData', localData));
    properties.add(DiagnosticsProperty('dataReader', dataReader));
  }

  @override
  Object? get localData => _item.localData;

  @override
  DataReader? get dataReader => _reader;

  @override
  List<PlatformFormat> get platformFormats => _item.formats;

  Future<void> _maybeInitReader() async {
    if (_reader == null && _item.readerItem != null) {
      _reader = await DataReader.forItem(_item.readerItem!);
    }
  }

  raw.DropItem _item;
  DataReader? _reader;
}

class _DropSession extends DropSession {
  @override
  List<DropItem> get items => _items;

  @override
  Listenable get onDisposed => _onDisposed;

  @override
  Set<raw.DropOperation> get allowedOperations => _allowedOperations;

  Future<void> updateItems(List<raw.DropItem> items) async {
    final current = List<_DropItem>.from(_items);
    _items.clear();

    for (final item in items) {
      final existing = current
          .firstWhereOrNull((element) => element._item.itemId == item.itemId);
      if (existing != null) {
        existing._item = item;
        _items.add(existing);
      } else {
        _items.add(_DropItem._(item));
      }
    }

    for (final item in _items) {
      await item._maybeInitReader();
    }
  }

  Future<raw.DropOperation> update({
    required ui.Offset position,
    required Set<raw.DropOperation> allowedOperations,
  }) async {
    _allowedOperations.clear();
    _allowedOperations.addAll(allowedOperations);
    _inside = true;

    final hitTest = HitTestResult();
    GestureBinding.instance.hitTest(hitTest, position);

    final monitorsInHitTest = <RenderDropMonitor>{};
    RenderDropRegion? dropRegion;

    var res = raw.DropOperation.none;

    for (final item in hitTest.path) {
      final target = item.target;
      if (target is RenderDropRegion && dropRegion == null) {
        res = await target.onDropOver(DropOverEvent(
          session: this,
          position: DropPosition.forRenderObject(position, target),
        ));
        if (res != raw.DropOperation.none) {
          dropRegion = target;
        }
      }
      if (target is RenderDropMonitor) {
        monitorsInHitTest.add(target);
      }
    }

    if (_currentDropRegion != dropRegion) {
      dropRegion?.onDropEnter?.call(DropEvent(session: this));
      _currentDropRegion?.onDropLeave?.call(DropEvent(session: this));
    }
    _currentDropRegion = dropRegion;
    if (_currentDropRegion != null) {
      _allRegions.add(_currentDropRegion!);
    }

    for (final monitor in RenderDropMonitor.activeMonitors) {
      final inside = monitorsInHitTest.contains(monitor);
      final dropPosition = DropPosition.forRenderObject(position, monitor);
      monitor.onDropOver?.call(
        MonitorDropOverEvent(
          session: this,
          position: dropPosition,
          isInside: inside,
        ),
      );
    }

    return res;
  }

  Future<void> performDrop({
    required ui.Offset location,
    required raw.DropOperation acceptedOperation,
  }) async {
    if (_currentDropRegion != null) {
      await _currentDropRegion?.onPerformDrop(PerformDropEvent(
          session: this,
          position: DropPosition.forRenderObject(location, _currentDropRegion!),
          acceptedOperation: acceptedOperation));
    }
  }

  void leave() {
    _currentDropRegion?.onDropLeave?.call(DropEvent(session: this));
    _currentDropRegion = null;
    _inside = false;
    _allowedOperations.clear();
    for (final monitor in RenderDropMonitor.activeMonitors) {
      monitor.onDropLeave?.call(DropEvent(session: this));
    }
  }

  void dispose() {
    if (_inside) {
      leave();
    }
    for (final region in _allRegions) {
      if (region.attached) {
        region.onDropEnded?.call(DropEvent(session: this));
      }
    }
    for (final monitor in RenderDropMonitor.activeMonitors) {
      monitor.onDropEnded?.call(DropEvent(session: this));
    }

    _onDisposed.notify();
  }

  Future<raw.ItemPreview?> getDropItemPreview(
      raw.ItemPreviewRequest request) async {
    final item = _items
        .firstWhereOrNull((element) => element._item.itemId == request.itemId);
    if (item != null && _currentDropRegion != null) {
      final req = _DropItemPreviewRequest(item: item, request: request);
      final response =
          await _currentDropRegion?.onGetDropItemPreview?.call(this, req);
      if (response != null) {
        final ratio = _currentDropRegion!.devicePixelRatio;
        return raw.ItemPreview(
          destinationRect: response.destinationRect,
          destinationImage: response.destinationImage != null
              ? await raw.ImageData.fromImage(
                  response.destinationImage!,
                  devicePixelRatio: ratio,
                )
              : null,
          fadeOutDelay: response.fadeOutDelay,
          fadeOutDuration: response.fadeOutDuration,
        );
      } else {
        return null;
      }
    } else {
      return null;
    }
  }

  @override
  void debugFillProperties(DiagnosticPropertiesBuilder properties) {
    super.debugFillProperties(properties);
    properties.add(DiagnosticsProperty('items', _items));
    properties
        .add(DiagnosticsProperty('allowedOperations', _allowedOperations));
    properties.defaultDiagnosticsTreeStyle = DiagnosticsTreeStyle.sparse;
  }

  RenderDropRegion? _currentDropRegion;
  final _allRegions = <RenderDropRegion>{};

  final _allowedOperations = <raw.DropOperation>{};
  bool _inside = false;
  final _onDisposed = SimpleNotifier();
  final _items = <_DropItem>[];
}

class _DropItemPreviewRequest extends DropItemPreviewRequest {
  _DropItemPreviewRequest({
    required this.item,
    required this.request,
  });

  @override
  final DropItem item;
  final raw.ItemPreviewRequest request;

  @override
  Duration get fadeOutDelay => request.fadeOutDelay;

  @override
  Duration get fadeOutDuration => request.fadeOutDuration;

  @override
  ui.Size get size => request.size;
}

class _DropContextDelegate extends raw.DropContextDelegate {
  @override
  Future<void> onDropEnded(raw.BaseDropEvent event) async {
    final session = _sessions.remove(event.sessionId);
    session?.dispose();
  }

  @override
  Future<void> onDropLeave(raw.BaseDropEvent event) async {
    _sessions[event.sessionId]?.leave();
  }

  @override
  Future<raw.DropOperation> onDropUpdate(raw.DropEvent event) async {
    final session =
        _sessions.putIfAbsent(event.sessionId, () => _DropSession());
    await session.updateItems(event.items);
    return session.update(
      position: event.locationInView,
      allowedOperations: Set.from(event.allowedOperations),
    );
  }

  @override
  Future<raw.ItemPreview?> onGetItemPreview(
      raw.ItemPreviewRequest request) async {
    final session = _sessions[request.sessionId];
    return session?.getDropItemPreview(request);
  }

  @override
  Future<void> onPerformDrop(raw.DropEvent event) async {
    final session = _sessions[event.sessionId];
    await session?.updateItems(event.items);
    await session?.performDrop(
      location: event.locationInView,
      acceptedOperation: event.acceptedOperation!,
    );
  }

  final _sessions = <int, _DropSession>{};
}

class DropFormatRegistry {
  DropFormatRegistry._();

  DropFormatRegistration registerFormats(List<DataFormat> dataFormats) {
    final platformFormats = <PlatformFormat>[];
    for (final dataFormat in dataFormats) {
      for (final format in dataFormat.decodingFormats) {
        if (!platformFormats.contains(format)) {
          platformFormats.add(format);
        }
      }
    }
    return registerPlatformDropFormats(platformFormats);
  }

  DropFormatRegistration registerPlatformDropFormats(
      List<PlatformFormat> formats) {
    final registration = DropFormatRegistration._(this);
    _registeredFormats[registration] = formats;
    _updateIfNeeded();
    return registration;
  }

  void _unregister(DropFormatRegistration registration) {
    _registeredFormats.remove(registration);
    _updateIfNeeded();
  }

  void _updateIfNeeded() async {
    final context = await raw.DropContext.instance();
    context.delegate ??= _DropContextDelegate();
    final formats = <PlatformFormat>{};
    for (final registration in _registeredFormats.values) {
      formats.addAll(registration);
    }
    final eq = const SetEquality().equals;
    if (_lastRegisteredFormats == null ||
        !eq(formats, _lastRegisteredFormats!)) {
      context.registerDropFormats(List.from(formats));
      _lastRegisteredFormats = formats;
    }
    // needed on some platforms (i.e. macOS)
    await raw.DragContext.instance();
  }

  static DropFormatRegistry instance = DropFormatRegistry._();

  final _registeredFormats = <DropFormatRegistration, List<PlatformFormat>>{};
  Set<PlatformFormat>? _lastRegisteredFormats;
}

class DropFormatRegistration {
  DropFormatRegistration._(this._registry);

  void dispose() {
    _registry._unregister(this);
  }

  final DropFormatRegistry _registry;
}

mixin RenderDropRegion on RenderObject {
  late FutureOr<raw.DropOperation> Function(DropOverEvent) onDropOver;
  void Function(DropEvent)? onDropEnter;
  void Function(DropEvent)? onDropLeave;
  late Future<void> Function(PerformDropEvent) onPerformDrop;
  void Function(DropEvent)? onDropEnded;
  OnGetDropItemPreview? onGetDropItemPreview;
  late double devicePixelRatio;

  DropFormatRegistration? _formatRegistration;

  void updateFormats(List<DataFormat> formats) {
    _formatRegistration?.dispose();
    _formatRegistration = DropFormatRegistry.instance.registerFormats(formats);
  }

  void _init({
    required List<DataFormat> formats,
    required FutureOr<raw.DropOperation> Function(DropOverEvent) onDropOver,
    required void Function(DropEvent)? onDropEnter,
    required void Function(DropEvent)? onDropLeave,
    required Future<void> Function(PerformDropEvent) onPerformDrop,
    required void Function(DropEvent)? onDropEnded,
    required OnGetDropItemPreview? onGetDropItemPreview,
    required double devicePixelRatio,
  }) {
    updateFormats(formats);
    this.onDropOver = onDropOver;
    this.onDropEnter = onDropEnter;
    this.onDropLeave = onDropLeave;
    this.onPerformDrop = onPerformDrop;
    this.onDropEnded = onDropEnded;
    this.onGetDropItemPreview = onGetDropItemPreview;
    this.devicePixelRatio = devicePixelRatio;
  }

  @override
  void dispose() {
    super.dispose();
    _formatRegistration?.dispose();
  }
}

class RenderDropRegionBox extends RenderProxyBoxWithHitTestBehavior
    with RenderDropRegion {
  RenderDropRegionBox({
    required super.behavior,
    required List<DataFormat> formats,
    required FutureOr<raw.DropOperation> Function(DropOverEvent) onDropOver,
    required void Function(DropEvent)? onDropEnter,
    required void Function(DropEvent)? onDropLeave,
    required Future<void> Function(PerformDropEvent) onPerformDrop,
    required void Function(DropEvent)? onDropEnded,
    required OnGetDropItemPreview? onGetDropItemPreview,
    required double devicePixelRatio,
  }) {
    _init(
      formats: formats,
      onDropOver: onDropOver,
      onDropEnter: onDropEnter,
      onDropLeave: onDropLeave,
      onPerformDrop: onPerformDrop,
      onDropEnded: onDropEnded,
      onGetDropItemPreview: onGetDropItemPreview,
      devicePixelRatio: devicePixelRatio,
    );
  }
}

class RenderDropRegionSliver extends RenderProxySliver with RenderDropRegion {
  RenderDropRegionSliver({
    required List<DataFormat> formats,
    required FutureOr<raw.DropOperation> Function(DropOverEvent) onDropOver,
    required void Function(DropEvent)? onDropEnter,
    required void Function(DropEvent)? onDropLeave,
    required Future<void> Function(PerformDropEvent) onPerformDrop,
    required void Function(DropEvent)? onDropEnded,
    required OnGetDropItemPreview? onGetDropItemPreview,
    required double devicePixelRatio,
  }) {
    _init(
      formats: formats,
      onDropOver: onDropOver,
      onDropEnter: onDropEnter,
      onDropLeave: onDropLeave,
      onPerformDrop: onPerformDrop,
      onDropEnded: onDropEnded,
      onGetDropItemPreview: onGetDropItemPreview,
      devicePixelRatio: devicePixelRatio,
    );
  }
}

mixin RenderDropMonitor on RenderObject {
  void Function(MonitorDropOverEvent)? onDropOver;
  void Function(DropEvent)? onDropLeave;
  void Function(DropEvent)? onDropEnded;

  static final activeMonitors = <RenderDropMonitor>{};

  DropFormatRegistration? _formatRegistration;

  void updateFormats(List<DataFormat> formats) {
    _formatRegistration?.dispose();
    _formatRegistration = DropFormatRegistry.instance.registerFormats(formats);
  }

  @override
  void dispose() {
    super.dispose();
    activeMonitors.remove(this);
    _formatRegistration?.dispose();
  }

  void _init({
    required List<DataFormat> formats,
    required void Function(MonitorDropOverEvent)? onDropOver,
    required void Function(DropEvent)? onDropLeave,
    required void Function(DropEvent)? onDropEnded,
  }) {
    updateFormats(formats);
    this.onDropOver = onDropOver;
    this.onDropLeave = onDropLeave;
    this.onDropEnded = onDropEnded;
    activeMonitors.add(this);
  }
}

class RenderDropMonitorBox extends RenderProxyBoxWithHitTestBehavior
    with RenderDropMonitor {
  RenderDropMonitorBox({
    required super.behavior,
    required List<DataFormat> formats,
    required void Function(MonitorDropOverEvent)? onDropOver,
    required void Function(DropEvent)? onDropLeave,
    required void Function(DropEvent)? onDropEnded,
  }) {
    _init(
      formats: formats,
      onDropOver: onDropOver,
      onDropLeave: onDropLeave,
      onDropEnded: onDropEnded,
    );
  }
}

class RenderDropMonitorSliver extends RenderProxySliver with RenderDropMonitor {
  RenderDropMonitorSliver({
    required List<DataFormat> formats,
    required void Function(MonitorDropOverEvent)? onDropOver,
    required void Function(DropEvent)? onDropLeave,
    required void Function(DropEvent)? onDropEnded,
  }) {
    _init(
      formats: formats,
      onDropOver: onDropOver,
      onDropLeave: onDropLeave,
      onDropEnded: onDropEnded,
    );
  }
}
