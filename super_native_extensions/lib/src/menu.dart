import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';

import 'api_model.dart';
import 'cancellation_token.dart';
import 'default_menu_image.dart';
import 'mutex.dart';

import 'native/menu.dart' if (dart.library.js) 'web/menu.dart';
import 'menu_flutter.dart';
import 'widgets/menu_widget/menu_widget_builder.dart';

abstract class MenuImage {
  /// If possible, returns the menu image represented as Widget.
  Widget? asWidget(IconThemeData theme);

  /// Returns image representation of the menu image.
  FutureOr<ui.Image?> asImage(
    IconThemeData theme,
    double devicePixelRatio,
  );

  MenuImage();

  /// Creates menu image for specified icon data.
  factory MenuImage.icon(IconData icon) => IconMenuImage(icon);

  /// Creates platform-specific image with given name.
  /// This currently works on iOS for SF symbol names
  /// (i.e. [UIImage systemImageNamed:]).
  factory MenuImage.system(String systemImageName) =>
      SystemMenuImage(systemImageName);

  /// Creates menu image from specified image provider function.
  factory MenuImage.withImage(
    FutureOr<ui.Image?>? Function(IconThemeData theme, int devicePixelRatio)
        imageProvider,
  ) =>
      ImageProviderMenuImage(imageProvider);
}

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
  final MenuImage? image;
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

class MenuActionAttributes {
  const MenuActionAttributes({
    this.disabled = false,
    this.destructive = false,
  });

  final bool disabled;
  final bool destructive;
}

enum MenuActionState {
  none,

  /// Checked status, supported on all platforms.
  checkOn,

  /// Should be used for unchecked checkbox as some platforms have special
  /// menu widget for checkable items that render differently to normal items.
  checkOff,

  /// Mixed check state, supported on iOS, Android and macOS.
  checkMixed,

  /// Supported on Windows, Android and Linux. Otherwise renders as [checkOn].
  radioOn,

  /// Unchecked radio item.
  radioOff,
}

class MenuAction extends MenuElement {
  MenuAction({
    super.title,
    super.image,
    super.identifier,
    required this.callback,
    this.attributes = const MenuActionAttributes(),
    this.state = MenuActionState.none,
  });

  final VoidCallback callback;
  final MenuActionAttributes attributes;
  final MenuActionState state;
}

class DeferredMenuElement extends MenuElement {
  DeferredMenuElement(this.provider);

  final Future<List<MenuElement>> Function(CancellationToken) provider;
}

class MenuConfiguration {
  final int configurationId;
  final TargetedImage liftImage;
  final ui.Image? previewImage;
  final ui.Size? previewSize;
  final MenuHandle handle;
  final MenuWidgetBuilder menuWidgetBuilder;
  final IconThemeData iconTheme;

  MenuConfiguration({
    required this.configurationId,
    required this.liftImage,
    this.previewImage,
    this.previewSize,
    required this.handle,
    required this.menuWidgetBuilder,
    required this.iconTheme,
  }) : assert(previewImage == null || previewSize == null,
            'previewImage and previewSize are mutually exclusive');
}

class MenuConfigurationRequest {
  final int configurationId;
  final ui.Offset location;
  final ValueSetter<ui.Image> previewImageSetter;

  MenuConfigurationRequest({
    required this.configurationId,
    required this.location,
    required this.previewImageSetter,
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

abstract class MenuContext {
  static final _mutex = Mutex();

  static MenuContext? _instance;

  MenuContextDelegate? delegate;

  Future<void> initialize();

  Future<MenuHandle> registerMenu(
    Menu menu,
    MenuSerializationOptions options,
  );

  static Future<MenuContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        if (!kIsWeb && defaultTargetPlatform == TargetPlatform.android) {
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

int _nextId = 1;
