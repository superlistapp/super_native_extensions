import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_context_menu/src/util.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

class MenuRequest {
  /// Invoked when menu is shown.
  final Listenable onShowMenu;

  /// Invoked when menu is hidden. This is will also be invoked when menu
  /// is dismissed before it is shown.
  final Listenable onHideMenu;

  /// iOS / Android only. Invoked when user taps on menu preview.
  final Listenable onPreviewAction;

  /// Menu location in global coordinates.
  final Offset location;

  MenuRequest({
    required this.onShowMenu,
    required this.onHideMenu,
    required this.onPreviewAction,
    required this.location,
  });
}

typedef MenuProvider = FutureOr<raw.Menu?> Function(MenuRequest request);

typedef MenuConfigurationProvider = Future<raw.MenuConfiguration?> Function(
    raw.MenuConfigurationRequest request);

class _MenuContextDelegate implements raw.MenuContextDelegate {
  static final instance = _MenuContextDelegate();

  @override
  Future<raw.MenuConfiguration?> getMenuConfiguration(
    raw.MenuConfigurationRequest request,
  ) async {
    final hitTest = HitTestResult();
    GestureBinding.instance.hitTest(hitTest, request.location);
    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderBaseContextMenu) {
        _onShow[request.configurationId] = [];
        _onHide[request.configurationId] = [];
        _onPreviewAction[request.configurationId] = [];
        final configuration = await target.getConfiguration(request);
        if (configuration == null) {
          _onShow.remove(request.configurationId);
          _onHide.remove(request.configurationId);
          _onPreviewAction.remove(request.configurationId);
        }
        return configuration;
      }
    }

    return null;
  }

  bool registerOnShowCallback(int configurationId, VoidCallback callback) {
    final callbacks = _onShow[configurationId];
    callbacks?.add(callback);
    return callbacks != null;
  }

  bool registerOnHideCallback(int configurationId, VoidCallback callback) {
    final callbacks = _onHide[configurationId];
    callbacks?.add(callback);
    return callbacks != null;
  }

  bool registerPreviewActionCallback(
      int configurationId, VoidCallback callback) {
    final callbacks = _onPreviewAction[configurationId];
    callbacks?.add(callback);
    return callbacks != null;
  }

  final _onShow = <int, List<VoidCallback>>{};
  final _onHide = <int, List<VoidCallback>>{};
  final _onPreviewAction = <int, List<VoidCallback>>{};

  @override
  void onHideMenu(int configurationId) {
    _onShow.remove(configurationId);
    _onPreviewAction.remove(configurationId);
    final onHide = _onHide.remove(configurationId);
    if (onHide != null) {
      for (final callback in onHide) {
        callback();
      }
    }
  }

  @override
  void onShowMenu(int configurationId) {
    final onShow = _onShow.remove(configurationId);
    if (onShow != null) {
      for (final callback in onShow) {
        callback();
      }
    }
  }

  @override
  void onPreviewAction(int configurationId) {
    final onPreviewAction = _onPreviewAction[configurationId];
    if (onPreviewAction != null) {
      for (final callback in onPreviewAction) {
        callback();
      }
    }
  }
}

typedef ContextMenuIsAllowed = bool Function(Offset location);

bool _defaultContextMenuIsAllowed(Offset location) => true;

class _SnapshotKey {
  _SnapshotKey(this.debugName);

  @override
  String toString() {
    return "SnapshotKey('$debugName') ${identityHashCode(this)}";
  }

  final String debugName;
}

final _keyLift = _SnapshotKey('Lift');
final _keyPreview = _SnapshotKey('Preview');

class DeferredMenuPreview {
  DeferredMenuPreview(this.size, this.widget);

  final Size size;
  final Future<Widget> widget;
}

class ContextMenuWidget extends StatefulWidget {
  ContextMenuWidget({
    super.key,
    this.liftBuilder,
    this.previewBuilder,
    this.deferredPreviewBuilder,
    required this.child,
    required this.hitTestBehavior,
    required this.menuProvider,
    this.iconTheme,
    this.contextMenuIsAllowed = _defaultContextMenuIsAllowed,
    MenuWidgetBuilder? menuWidgetBuilder,
  })  : assert(previewBuilder == null || deferredPreviewBuilder == null,
            'Cannot use both previewBuilder and deferredPreviewBuilder'),
        menuWidgetBuilder = menuWidgetBuilder ?? DefaultMenuWidgetBuilder();

  final Widget Function(BuildContext context, Widget child)? liftBuilder;
  final Widget Function(BuildContext context, Widget child)? previewBuilder;
  final DeferredMenuPreview Function(BuildContext context, Widget child,
      raw.CancellationToken cancellationToken)? deferredPreviewBuilder;
  final HitTestBehavior hitTestBehavior;
  final MenuProvider menuProvider;
  final ContextMenuIsAllowed contextMenuIsAllowed;
  final Widget child;
  final MenuWidgetBuilder menuWidgetBuilder;

  /// Base icon theme for menu icons. The size will be overriden depending
  /// on platform.
  final IconThemeData? iconTheme;

  @override
  State<ContextMenuWidget> createState() => _ContextMenuWidgetState();
}

class _ContextMenuWidgetState extends State<ContextMenuWidget> {
  MenuSerializationOptions _serializationOptions(BuildContext context) {
    final mq = MediaQuery.of(context);
    final iconTheme = widget.iconTheme ??
        const IconThemeData.fallback().copyWith(
          color: mq.platformBrightness == Brightness.light
              ? const Color(0xFF090909)
              : const Color(0xFFF0F0F0),
        );
    return MenuSerializationOptions(
      iconTheme,
      mq.devicePixelRatio,
    );
  }

  Future<raw.MenuConfiguration?> getMenuConfiguration(
      raw.MenuConfigurationRequest request) async {
    if (!widget.contextMenuIsAllowed(request.location)) {
      return null;
    }

    final onShowMenu = SimpleNotifier();
    final onHideMenu = SimpleNotifier();
    final onPreviewAction = SimpleNotifier();
    final menu = await widget.menuProvider(MenuRequest(
      onShowMenu: onShowMenu,
      onHideMenu: onHideMenu,
      onPreviewAction: onPreviewAction,
      location: request.location,
    ));
    final snapshotter = _snapshotterKey.currentState!;
    if (menu != null && mounted) {
      final liftImage = await snapshotter.getSnapshot(request.location,
          _keyLift, () => widget.liftBuilder?.call(context, widget.child));

      if (liftImage == null) {
        // might happen if the widget was removed from hierarchy.
        onHideMenu.notify();
        return null;
      }

      final previewImage = widget.previewBuilder != null
          ? await snapshotter.getSnapshot(request.location, _keyPreview,
              () => widget.previewBuilder!.call(context, widget.child))
          : null;

      final menuContext = await raw.MenuContext.instance();

      if (!mounted) {
        return null;
      }

      final serializationOptions = _serializationOptions(context);
      final handle = await menuContext.registerMenu(
        menu,
        serializationOptions,
      );

      _MenuContextDelegate.instance.registerOnHideCallback(
        request.configurationId,
        () {
          onHideMenu.notify();
          handle.dispose();
        },
      );

      _MenuContextDelegate.instance.registerOnShowCallback(
        request.configurationId,
        onShowMenu.notify,
      );

      _MenuContextDelegate.instance.registerPreviewActionCallback(
        request.configurationId,
        onPreviewAction.notify,
      );

      Size? deferredSize = widget.deferredPreviewBuilder != null
          ? _getDeferredPreview(onHideMenu, request.previewImageSetter)
          : null;

      return raw.MenuConfiguration(
        configurationId: request.configurationId,
        liftImage: liftImage,
        previewImage: previewImage?.image,
        previewSize: deferredSize,
        handle: handle,
        menuWidgetBuilder: widget.menuWidgetBuilder,
        iconTheme: serializationOptions.iconTheme,
      );
    } else {
      return null;
    }
  }

  Size _getDeferredPreview(
      Listenable onHide, ValueSetter<ui.Image> imageSetter) {
    final cancellationToken = raw.SimpleCancellationToken();
    onHide.addListener(cancellationToken.cancel);
    final deferredPreview = widget.deferredPreviewBuilder!(
        context, widget.child, cancellationToken);
    deferredPreview.widget.then((widget) {
      if (!cancellationToken.cancelled) {
        cancellationToken.dispose();
        _updateMenuPreview(widget, deferredPreview.size, imageSetter);
      }
    }, onError: (error) {
      cancellationToken.dispose();
    });

    return deferredPreview.size;
  }

  void _updateMenuPreview(
      Widget preview, Size size, ValueSetter<ui.Image> imageSetter) async {
    final snapshotter = _snapshotterKey.currentState!;
    final child = SnapshotSettings(
      constraintsTransform: (_) => BoxConstraints.tight(size),
      child: preview,
    );
    final previewImage = await snapshotter.getSnapshot(
      Offset.zero,
      _SnapshotKey(
          'DeferredPreview'), // Deferred preview must have separate key.
      () => child,
    );
    if (previewImage != null) {
      imageSetter(previewImage.image);
    }
  }

  final _snapshotterKey = GlobalKey<WidgetSnapshotterState>();

  @override
  Widget build(BuildContext context) {
    return WidgetSnapshotter(
      key: _snapshotterKey,
      child: Listener(
        behavior: HitTestBehavior.translucent,
        onPointerDown: (_) {
          if (defaultTargetPlatform == TargetPlatform.iOS ||
              defaultTargetPlatform == TargetPlatform.android) {
            {
              _snapshotterKey.currentState?.registerWidget(
                  _keyLift,
                  widget.liftBuilder?.call(
                    context,
                    widget.child,
                  ));
              if (widget.previewBuilder != null) {
                _snapshotterKey.currentState?.registerWidget(
                    _keyPreview,
                    widget.previewBuilder!.call(
                      context,
                      widget.child,
                    ));
              }
            }
          }
        },
        onPointerCancel: (_) {
          _snapshotterKey.currentState?.unregisterWidget(_keyLift);
          _snapshotterKey.currentState?.unregisterWidget(_keyPreview);
        },
        onPointerUp: (_) {
          _snapshotterKey.currentState?.unregisterWidget(_keyLift);
          _snapshotterKey.currentState?.unregisterWidget(_keyPreview);
        },
        child: BaseContextMenuRenderWidget(
          hitTestBehavior: widget.hitTestBehavior,
          getConfiguration: getMenuConfiguration,
          child: widget.child,
        ),
      ),
    );
  }
}

class BaseContextMenuRenderWidget extends SingleChildRenderObjectWidget {
  const BaseContextMenuRenderWidget({
    super.key,
    required super.child,
    required this.hitTestBehavior,
    required this.getConfiguration,
  });

  final HitTestBehavior hitTestBehavior;
  final MenuConfigurationProvider getConfiguration;

  @override
  RenderObject createRenderObject(BuildContext context) {
    _initializeIfNeeded();
    return _RenderBaseContextMenu(
      behavior: hitTestBehavior,
      getConfiguration: getConfiguration,
    );
  }

  @override
  void updateRenderObject(
    BuildContext context,
    covariant RenderObject renderObject,
  ) {
    final renderObject_ = renderObject as _RenderBaseContextMenu;
    renderObject_.behavior = hitTestBehavior;
    renderObject_.getConfiguration = getConfiguration;
  }
}

class _RenderBaseContextMenu extends RenderProxyBoxWithHitTestBehavior {
  _RenderBaseContextMenu({
    required super.behavior,
    required this.getConfiguration,
  });

  MenuConfigurationProvider getConfiguration;
}

bool _initialized = false;

void _initializeIfNeeded() async {
  if (!_initialized) {
    _initialized = true;
    final menuContext = await raw.MenuContext.instance();
    menuContext.delegate = _MenuContextDelegate.instance;
  }
}
