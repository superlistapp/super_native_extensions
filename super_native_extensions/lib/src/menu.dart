import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'drag.dart';
import 'menu_model.dart';
import 'mutex.dart';

import 'native/menu.dart' if (dart.library.js) 'web/menu.dart';
import 'menu_flutter.dart';
import 'widget_snapshot/widget_snapshot.dart';

abstract class MobileMenuDelegate {
  void didPushSubmenu();
  void hideMenu({required bool itemSelected});
}

typedef MobileMenuWidgetFactory = Widget Function(
  BuildContext context,
  Menu rootMenu,
  MobileMenuDelegate delegate,
  AlignmentGeometry alignment,
  ValueListenable<bool> canScrollListenable,
  IconThemeData iconTheme,
);

class MobileMenuConfiguration {
  MobileMenuConfiguration({
    required this.configurationId,
    required this.liftImage,
    this.previewImage,
    this.previewSize,
    required this.handle,
    required this.backgroundBuilder,
    required this.previewBuilder,
    required this.menuWidgetBuilder,
    required this.iconTheme,
  }) : assert(previewImage == null || previewSize == null,
            'previewImage and previewSize are mutually exclusive');

  final int configurationId;
  final TargetedWidgetSnapshot liftImage;
  final WidgetSnapshot? previewImage;
  final ui.Size? previewSize;
  final MenuHandle handle;
  final IconThemeData iconTheme;

  final Widget Function(double opacity) backgroundBuilder;
  final Widget Function(Size, WidgetSnapshot?) previewBuilder;
  final MobileMenuWidgetFactory menuWidgetBuilder;

  void dispose() {
    liftImage.dispose();
    previewImage?.dispose();
  }
}

class MobileMenuConfigurationRequest {
  final int configurationId;
  final ui.Offset location;
  final ValueSetter<WidgetSnapshot> previewImageSetter;

  MobileMenuConfigurationRequest({
    required this.configurationId,
    required this.location,
    required this.previewImageSetter,
  });
}

abstract class MenuContextDelegate {
  Future<MobileMenuConfiguration?> getMenuConfiguration(
    MobileMenuConfigurationRequest request,
  );
  void onShowMenu(int configurationId);
  void onHideMenu(int configurationId, MenuResult response);
  void onPreviewAction(int configurationId);
  bool contextMenuIsAllowed(Offset location);
}

abstract class MenuHandle {
  Menu get menu;
  void dispose();
}

class MenuSerializationOptions {
  MenuSerializationOptions(
    this.iconTheme,
    this.devicePixelRatio,
  );

  final IconThemeData iconTheme;
  final double devicePixelRatio;
}

class DesktopContextMenuRequest {
  DesktopContextMenuRequest({
    required this.menu,
    required this.position,
    required this.iconTheme,
    required this.fallback,
  });

  final MenuHandle menu;
  final Offset position;
  final IconThemeData iconTheme;

  // Passed to delegate when requesting Flutter desktop menu implementation.
  final Future<MenuResult> Function() fallback;
}

abstract class MenuContext {
  static final _mutex = Mutex();

  static MenuContext? _instance;

  MenuContextDelegate? delegate;

  static bool get isTouchDevice => DragContext.isTouchDevice;

  Future<void> initialize();

  Future<MenuResult> showContextMenu(DesktopContextMenuRequest request);

  Future<MenuHandle> registerMenu(
    Menu menu,
    MenuSerializationOptions options,
  );

  static Future<MenuContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        if (!kIsWeb &&
            (defaultTargetPlatform == TargetPlatform.android ||
                defaultTargetPlatform == TargetPlatform.windows)) {
          _instance = FlutterMenuContext();
        } else {
          _instance = MenuContextImpl();
        }
        await _instance!.initialize();
      }
      return _instance!;
    });
  }
}
