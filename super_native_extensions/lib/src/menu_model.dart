import 'dart:ui' as ui;
import 'dart:async';

import 'package:flutter/widgets.dart';

import 'cancellation_token.dart';
import 'menu_image_impl.dart';

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
    this.title,
    this.subtitle,
    this.image,
  }) : uniqueId = _nextId++;

  final String? title;
  final String? subtitle;
  final MenuImage? image;
  final int uniqueId;

  MenuElement? find({required int uniqueId}) {
    if (uniqueId == this.uniqueId) {
      return this;
    } else {
      return null;
    }
  }
}

class MenuSeparator extends MenuElement {
  MenuSeparator({super.title});
}

class Menu extends MenuElement {
  Menu({
    super.title,
    super.image,
    required this.children,
  });

  final List<MenuElement> children;

  @override
  MenuElement? find({required int uniqueId}) {
    final result = super.find(uniqueId: uniqueId);
    if (result != null) {
      return result;
    } else {
      for (final child in children) {
        final result = child.find(uniqueId: uniqueId);
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

  /// Mixed check state, supported on all platforms.
  checkMixed,

  /// Supported on Windows, Android, Linux and Web. Otherwise renders as [checkOn].
  radioOn,

  /// Unchecked radio item.
  radioOff,
}

class MenuAction extends MenuElement {
  MenuAction({
    super.title,
    super.image,
    required this.callback,
    this.attributes = const MenuActionAttributes(),
    this.state = MenuActionState.none,
    this.activator,
  });

  final VoidCallback callback;
  final MenuActionAttributes attributes;
  final MenuActionState state;
  final SingleActivator? activator;
}

class DeferredMenuElement extends MenuElement {
  DeferredMenuElement(this.provider);

  final Future<List<MenuElement>> Function(CancellationToken) provider;
}

class MenuResult {
  MenuResult({
    required this.itemSelected,
  });

  /// Whether any menu item was selected from this menu. If false, the menu
  /// was dismissed without selecting any item.
  final bool itemSelected;
}

int _nextId = 1;
