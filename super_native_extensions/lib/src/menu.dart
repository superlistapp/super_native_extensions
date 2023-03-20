import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';

import 'api_model.dart';
import 'mutex.dart';
import 'native/menu.dart';

class MenuElement {
  MenuElement({
    this.identifier,
    this.title,
    this.subtitle,
    this.image,
  }) : uniqueId = _nextId++;

  final String? identifier;
  final String? title;
  final String? subtitle;
  final FutureOr<ui.Image?>? image;
  final int uniqueId;

  MenuElement? find({int? uniqueId, String? identifier}) {
    assert(uniqueId != null || identifier != null);
    if (uniqueId != null && uniqueId == this.uniqueId) {
      return this;
    } else if (identifier != null && identifier == this.identifier) {
      return this;
    } else {
      return null;
    }
  }
}

class Separator extends MenuElement {
  Separator({super.title});
}

class Menu extends MenuElement {
  Menu({
    super.identifier,
    super.title,
    super.image,
    required this.children,
  });

  final List<MenuElement> children;

  @override
  MenuElement? find({int? uniqueId, String? identifier}) {
    final result = super.find(uniqueId: uniqueId, identifier: identifier);
    if (result != null) {
      return result;
    } else {
      for (final child in children) {
        final result = child.find(uniqueId: uniqueId, identifier: identifier);
        if (result != null) {
          return result;
        }
      }
      return null;
    }
  }
}

class MenuElementAttributes {
  const MenuElementAttributes({
    this.disabled = false,
    this.destructive = false,
  });

  final bool disabled;
  final bool destructive;
}

class MenuAction extends MenuElement {
  MenuAction({
    super.title,
    super.image,
    super.identifier,
    required this.callback,
    this.attributes = const MenuElementAttributes(),
  });

  final VoidCallback callback;
  final MenuElementAttributes attributes;
}

class DeferredMenuElement extends MenuElement {
  DeferredMenuElement(this.provider);

  final AsyncValueGetter<List<MenuElement>> provider;
}

class MenuConfiguration {
  final int configurationId;
  final TargetedImageData image;
  final TargetedImageData? liftImage;
  final MenuHandle handle;

  MenuConfiguration({
    required this.configurationId,
    required this.image,
    required this.handle,
    this.liftImage,
  });
}

class MenuConfigurationRequest {
  final int configurationId;
  final ui.Offset location;

  MenuConfigurationRequest({
    required this.configurationId,
    required this.location,
  });
}

abstract class MenuContextDelegate {
  Future<MenuConfiguration?> getMenuConfiguration(
    MenuConfigurationRequest request,
  );
  void onShowMenu(int configurationId);
  void onHideMenu(int configurationId);
  void onPreviewAction(int configurationId);
}

abstract class MenuHandle {
  void dispose();
}

abstract class MenuContext {
  static final _mutex = Mutex();

  static MenuContextImpl? _instance;

  MenuContextDelegate? delegate;

  Future<MenuHandle> registerMenu(Menu menu);

  static Future<MenuContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = MenuContextImpl();
        await _instance!.initialize();
      }
      return _instance!;
    });
  }
}

int _nextId = 1;
