import 'dart:async';

import 'package:flutter/gestures.dart';
import 'package:flutter/rendering.dart';
import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

import 'menu.dart';
import 'menu_model.dart';

class MenuContextDelegate implements raw.MenuContextDelegate {
  static final instance = MenuContextDelegate();

  @override
  Future<MobileMenuConfiguration?> getMenuConfiguration(
    MobileMenuConfigurationRequest request,
  ) async {
    final hitTest = HitTestResult();
    // TODO(knopp): Resolve when we can provide viewId from native side
    // ignore: deprecated_member_use
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
          continue;
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

  bool registerOnHideCallback(
      int configurationId, ValueSetter<MenuResult> callback) {
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
  final _onHide = <int, List<ValueSetter<MenuResult>>>{};
  final _onPreviewAction = <int, List<VoidCallback>>{};

  @override
  void onHideMenu(int configurationId, MenuResult response) {
    _onShow.remove(configurationId);
    _onPreviewAction.remove(configurationId);
    final onHide = _onHide.remove(configurationId);
    if (onHide != null) {
      for (final callback in onHide) {
        callback(response);
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

  @override
  bool contextMenuIsAllowed(Offset location) {
    final hitTest = HitTestResult();
    // TODO(knopp): Resolve when we can provide viewId from native side
    // ignore: deprecated_member_use
    GestureBinding.instance.hitTest(hitTest, location);
    for (final item in hitTest.path) {
      final target = item.target;
      if (target is _RenderBaseContextMenu) {
        return target.contextMenuIsAllowed(location);
      }
    }
    return false;
  }
}

class BaseContextMenuRenderWidget extends SingleChildRenderObjectWidget {
  const BaseContextMenuRenderWidget({
    super.key,
    required super.child,
    required this.hitTestBehavior,
    required this.getConfiguration,
    required this.contextMenuIsAllowed,
  });

  final HitTestBehavior hitTestBehavior;
  final MenuConfigurationProvider getConfiguration;
  final ContextMenuIsAllowed contextMenuIsAllowed;

  @override
  RenderObject createRenderObject(BuildContext context) {
    _initializeIfNeeded();
    return _RenderBaseContextMenu(
      behavior: hitTestBehavior,
      getConfiguration: getConfiguration,
      contextMenuIsAllowed: contextMenuIsAllowed,
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
    renderObject_.contextMenuIsAllowed = contextMenuIsAllowed;
  }
}

class _RenderBaseContextMenu extends RenderProxyBoxWithHitTestBehavior {
  _RenderBaseContextMenu({
    required super.behavior,
    required this.getConfiguration,
    required this.contextMenuIsAllowed,
  });

  ContextMenuIsAllowed contextMenuIsAllowed;
  MenuConfigurationProvider getConfiguration;
}

bool _initialized = false;

raw.LongPressHandler? longPressHandler;

void _initializeIfNeeded() async {
  if (!_initialized) {
    _initialized = true;
    final menuContext = await raw.MenuContext.instance();
    longPressHandler = await raw.LongPressHandler.create();
    menuContext.delegate = MenuContextDelegate.instance;
  }
}
