import 'dart:ui' as ui;
import 'package:collection/collection.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_clipboard/super_clipboard.dart';
import 'package:super_native_extensions/raw_drag_drop.dart' as raw;

import 'util.dart';

class DropItem {
  DropItem._(this._item);

  List<String> get formats => _item.formats;
  Object? get localData => _item.localData;

  DataReader? get dataReader =>
      _item.readerItem != null ? DataReader.forItem(_item.readerItem!) : null;

  raw.DropItem _item;
}

class DropSession {
  List<DropItem> get items => _items;

  Listenable get onDisposed => _onDisposed;

  Set<raw.DropOperation> get allowedOperations => _allowedOperations;

  void _updateItems(List<raw.DropItem> items) {
    final current = List<DropItem>.from(_items);
    _items.clear();

    for (final item in items) {
      final existing = current
          .firstWhereOrNull((element) => element._item.itemId == item.itemId);
      if (existing != null) {
        existing._item = item;
      } else {
        _items.add(DropItem._(item));
      }
    }
  }

  Future<raw.DropOperation> _update({
    required ui.Offset globalPosition,
    required Set<raw.DropOperation> allowedOperations,
  }) async {
    _allowedOperations.clear();
    _allowedOperations.addAll(allowedOperations);
    _inside = true;
    return raw.DropOperation.none;
  }

  Future<void> _performDrop({
    required ui.Offset globalPosition,
    required raw.DropOperation acceptedOperation,
  }) async {}

  void _leave() {
    _inside = false;
    _allowedOperations.clear();
    for (final monitor in _RenderDropMonitor._activeMonitors) {
      monitor.onLeave(this);
    }
  }

  void _dispose() {
    if (_inside) {
      _leave();
    }
    _onDisposed.notify();
  }

  final _allowedOperations = <raw.DropOperation>{};
  bool _inside = false;
  final _onDisposed = SimpleNotifier();
  final _items = <DropItem>[];
}

class _DropContextDelegate extends raw.DropContextDelegate {
  @override
  Future<void> onDropEnded(raw.BaseDropEvent event) async {
    final session = _sessions.remove(event.sessionId);
    session?._dispose();
  }

  @override
  Future<void> onDropLeave(raw.BaseDropEvent event) async {
    _sessions[event.sessionId]?._leave();
  }

  @override
  Future<raw.DropOperation> onDropUpdate(raw.DropEvent event) {
    final session = _sessions.putIfAbsent(event.sessionId, () => DropSession());
    session._updateItems(event.items);
    return session._update(
      globalPosition: event.locationInView,
      allowedOperations: Set.from(event.allowedOperations),
    );
  }

  @override
  Future<raw.ItemPreview?> onGetItemPreview(
      raw.ItemPreviewRequest request) async {
    return null;
  }

  @override
  Future<void> onPerformDrop(raw.DropEvent event) async {
    final session = _sessions[event.sessionId];
    session?._performDrop(
      globalPosition: event.locationInView,
      acceptedOperation: event.acceptedOperation!,
    );
  }

  final _sessions = <int, DropSession>{};
}

class DropFormatRegistry {
  DropFormatRegistry._();

  DropFormatRegistration registerFormats(
      List<EncodableDataFormat> dataFormats) {
    final platformFormats = <PlatformFormat>[];
    for (final dataFormat in dataFormats) {
      for (final format in dataFormat.decodableFormats) {
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
    if (formats != _lastRegisteredFormat) {
      context.registerDropFormats(List.from(formats));
      _lastRegisteredFormat.clear();
      _lastRegisteredFormat.addAll(formats);
    }
  }

  static DropFormatRegistry instance = DropFormatRegistry._();

  final _registeredFormats = <DropFormatRegistration, List<PlatformFormat>>{};
  final _lastRegisteredFormat = <PlatformFormat>{};
}

class DropFormatRegistration {
  DropFormatRegistration._(this._registry);

  void dispose() {
    _registry._unregister(this);
  }

  final DropFormatRegistry _registry;
}

typedef OnDropOver = Future<raw.DropOperation> Function(
  DropSession session,
  Offset location,
);

typedef OnDropLeave = Function(DropSession session);

typedef OnPerformDrop = Future<void> Function(
  DropSession session,
  Offset location,
  raw.DropOperation acceptedOperation,
);

class RawDropRegion {}

class DropMonitor extends SingleChildRenderObjectWidget {
  const DropMonitor({
    super.key,
    super.child,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    required this.formats,
  });

  final HitTestBehavior hitTestBehavior;
  final List<EncodableDataFormat> formats;

  @override
  RenderObject createRenderObject(BuildContext context) {
    return _RenderDropMonitor(behavior: hitTestBehavior, formats: formats);
  }

  @override
  void updateRenderObject(
      BuildContext context, covariant RenderObject renderObject_) {
    final renderObject = renderObject_ as _RenderDropMonitor;
    renderObject.behavior = hitTestBehavior;
    renderObject._formatRegistration.dispose();
    renderObject._formatRegistration =
        DropFormatRegistry.instance.registerFormats(formats);
  }
}

class _RenderDropMonitor extends RenderProxyBoxWithHitTestBehavior {
  static final _activeMonitors = <_RenderDropMonitor>{};
  DropFormatRegistration _formatRegistration;

  _RenderDropMonitor({
    super.child,
    required super.behavior,
    required List<EncodableDataFormat> formats,
  }) : _formatRegistration =
            DropFormatRegistry.instance.registerFormats(formats) {
    _activeMonitors.add(this);
  }

  void onLeave(DropSession session) {}

  @override
  void dispose() {
    _activeMonitors.remove(this);
    super.dispose();
  }
}
