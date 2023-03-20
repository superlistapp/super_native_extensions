import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_context_menu/src/util.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;
import 'package:super_native_extensions/widgets.dart';

class MenuRequest {
  final Listenable onShowMenu;
  final Listenable onHideMenu;
  final Listenable onPreviewAction;
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
    MenuConfigurationRequest request);

class _MenuContextDelegate implements raw.MenuContextDelegate {
  static final instance = _MenuContextDelegate();

  @override
  Future<raw.MenuConfiguration?> getMenuConfiguration(
    MenuConfigurationRequest request,
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

class ContextMenuWidget extends StatefulWidget {
  const ContextMenuWidget({
    super.key,
    required this.child,
    required this.hitTestBehavior,
    required this.menuProvider,
    this.contextMenuIsAllowed = _defaultContextMenuIsAllowed,
  });

  final HitTestBehavior hitTestBehavior;
  final MenuProvider menuProvider;
  final ContextMenuIsAllowed contextMenuIsAllowed;
  final Widget child;

  @override
  State<ContextMenuWidget> createState() => _ContextMenuWidgetState();
}

class _ContextMenuWidgetState extends State<ContextMenuWidget> {
  Future<MenuConfiguration?> getMenuConfiguration(
      MenuConfigurationRequest request) async {
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
    if (menu != null && mounted) {
      final devicePixelRatio = MediaQuery.of(context).devicePixelRatio;

      final snapshotter = Snapshotter.of(_innerContext!)!;
      final menuSnapshot =
          await snapshotter.getSnapshot(request.location, SnapshotType.menu);
      final snapshot =
          menuSnapshot ?? await snapshotter.getSnapshot(request.location, null);

      if (snapshot == null) {
        // might happen if the widget was removed from hierarchy.
        onHideMenu.notify();
        return null;
      }

      raw.TargetedImage? liftImage;
      if (defaultTargetPlatform == TargetPlatform.iOS) {
        liftImage =
            await snapshotter.getSnapshot(request.location, SnapshotType.lift);
        // If there is no custom lift image but custom drag snapshot, use
        // default image as lift image for smoother transition.
        if (liftImage == null && menuSnapshot != null) {
          liftImage = await snapshotter.getSnapshot(request.location, null);
        }
      }

      final menuContext = await MenuContext.instance();
      final handle = await menuContext.registerMenu(menu);

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

      return MenuConfiguration(
        configurationId: request.configurationId,
        image: await snapshot.intoRaw(devicePixelRatio),
        liftImage: await liftImage?.intoRaw(devicePixelRatio),
        handle: handle,
      );
    } else {
      return null;
    }
  }

  BuildContext? _innerContext;

  @override
  Widget build(BuildContext context) {
    return FallbackSnapshotWidget(
      child: Builder(builder: (context) {
        _innerContext = context;
        return Listener(
          behavior: HitTestBehavior.translucent,
          onPointerDown: (_) {
            if (defaultTargetPlatform == TargetPlatform.iOS) {
              Snapshotter.of(context)?.prepare({
                SnapshotType.lift,
                SnapshotType.menu,
              });
            }
          },
          onPointerCancel: (_) {
            if (defaultTargetPlatform == TargetPlatform.iOS &&
                context.mounted) {
              Snapshotter.of(context)?.unprepare();
            }
          },
          onPointerUp: (_) {
            if (defaultTargetPlatform == TargetPlatform.iOS &&
                context.mounted) {
              Snapshotter.of(context)?.unprepare();
            }
          },
          child: BaseContextMenuRenderWidget(
            hitTestBehavior: widget.hitTestBehavior,
            getConfiguration: getMenuConfiguration,
            child: widget.child,
          ),
        );
      }),
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
