import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/gestures.dart';
import 'package:flutter/widgets.dart';
import 'package:flutter_keyboard_visibility/flutter_keyboard_visibility.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

import 'default_builder/desktop_menu_widget_builder.dart';
import 'default_builder/mobile_menu_widget_builder.dart';
import 'desktop.dart';
import 'menu_model.dart';
import 'mobile.dart';
import 'scaffold/desktop/menu_widget_builder.dart';
import 'scaffold/mobile/menu_widget_builder.dart';

class MenuRequest {
  /// Invoked when menu is shown.
  final Listenable onShowMenu;

  /// Invoked when menu is hidden. This is will also be invoked when menu
  /// is dismissed before it is shown.
  final ValueListenable<MenuResult?> onHideMenu;

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

typedef MenuProvider = FutureOr<Menu?> Function(MenuRequest request);

typedef MenuConfigurationProvider = Future<MobileMenuConfiguration?> Function(
    MobileMenuConfigurationRequest request);

class DeferredMenuPreview {
  DeferredMenuPreview(this.size, this.widget);

  final Size size;
  final Future<Widget> widget;
}

typedef ContextMenuIsAllowed = bool Function(Offset location);

class ContextMenuWidget extends StatelessWidget {
  ContextMenuWidget({
    super.key,
    this.liftBuilder,
    this.previewBuilder,
    this.deferredPreviewBuilder,
    required this.child,
    this.hitTestBehavior = HitTestBehavior.deferToChild,
    required this.menuProvider,
    this.iconTheme,
    this.onMenuShown,
    this.onMenuHidden,
    this.shouldReopenKeyboard = false,
    this.contextMenuIsAllowed = _defaultContextMenuIsAllowed,
    MobileMenuWidgetBuilder? mobileMenuWidgetBuilder,
    DesktopMenuWidgetBuilder? desktopMenuWidgetBuilder,
    this.writingToolsConfigurationProvider,
  })  : assert(previewBuilder == null || deferredPreviewBuilder == null,
            'Cannot use both previewBuilder and deferredPreviewBuilder'),
        mobileMenuWidgetBuilder =
            mobileMenuWidgetBuilder ?? DefaultMobileMenuWidgetBuilder.instance,
        desktopMenuWidgetBuilder =
            desktopMenuWidgetBuilder ?? DefaultDesktopMenuWidgetBuilder();

  final Widget Function(BuildContext context, Widget child)? liftBuilder;
  final Widget Function(BuildContext context, Widget child)? previewBuilder;
  final DeferredMenuPreview Function(BuildContext context, Widget child,
      CancellationToken cancellationToken)? deferredPreviewBuilder;

  final HitTestBehavior hitTestBehavior;
  final MenuProvider menuProvider;
  final ContextMenuIsAllowed contextMenuIsAllowed;
  final Widget child;
  final MobileMenuWidgetBuilder mobileMenuWidgetBuilder;
  final DesktopMenuWidgetBuilder desktopMenuWidgetBuilder;
  final VoidCallback? onMenuShown;
  final VoidCallback? onMenuHidden;

  ///Works only on Android, default is false.
  final bool shouldReopenKeyboard;

  final WritingToolsConfiguration? Function()?
      writingToolsConfigurationProvider;

  /// Base icon theme for menu icons. The size will be overridden depending
  /// on platform.
  final IconThemeData? iconTheme;

  @override
  Widget build(BuildContext context) {
    final kind = raw.PointerDeviceKindDetector.instance.current;
    return ListenableBuilder(
      listenable: kind,
      child: child,
      builder: (context, child) {
        if (kind.value == PointerDeviceKind.touch) {
          return KeyboardVisibilityProvider(
            child: MobileContextMenuWidget(
            hitTestBehavior: hitTestBehavior,
            menuProvider: menuProvider,
            liftBuilder: liftBuilder,
            previewBuilder: previewBuilder,
            deferredPreviewBuilder: deferredPreviewBuilder,
            iconTheme: iconTheme,
            contextMenuIsAllowed: contextMenuIsAllowed,
            menuWidgetBuilder: mobileMenuWidgetBuilder,
            onMenuShown: onMenuShown,
            onMenuHidden: onMenuHidden,
            shouldReopenKeyboard: shouldReopenKeyboard,
            child: child!,),
          );
        } else {
          return DesktopContextMenuWidget(
            hitTestBehavior: hitTestBehavior,
            menuProvider: menuProvider,
            contextMenuIsAllowed: contextMenuIsAllowed,
            iconTheme: iconTheme,
            menuWidgetBuilder: desktopMenuWidgetBuilder,
            writingToolsConfigurationProvider:
                writingToolsConfigurationProvider,
            child: child!,
          );
        }
      },
    );
  }
}

bool _defaultContextMenuIsAllowed(Offset location) => true;

class WritingToolsConfiguration {
  WritingToolsConfiguration({
    required this.text,
    required this.rect,
    required this.onSuggestion,
  });

  final String text;
  final Rect rect;
  final ValueChanged<String> onSuggestion;
}
