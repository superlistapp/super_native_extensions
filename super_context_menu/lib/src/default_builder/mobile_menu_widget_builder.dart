import 'dart:ui' as ui;

import 'package:collection/collection.dart';
import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart'
    show Colors, Icons, CircularProgressIndicator, Scrollbar;
import 'package:pixel_snap/widgets.dart';

import '../menu_model.dart';
import '../scaffold/mobile/menu_widget_builder.dart';

class MobileMenuItemInfo {
  MobileMenuItemInfo({
    required this.isHeader,
    required this.isPressed,
    required this.isLast,
    required this.isDisabled,
    required this.isDestructive,
  });

  final bool isHeader;
  final bool isPressed;
  final bool isLast;
  final bool isDisabled;
  final bool isDestructive;
}

class DefaultMobileMenuTheme {
  DefaultMobileMenuTheme({
    required this.menuDecorationOutside,
    required this.menuDecorationInside,
    required this.menuPreviewDecorationOutside,
    required this.menuPreviewDecorationInside,
    required this.backgroundTintColor,
    required this.inactiveMenuVeilColor,
    required this.separatorColor,
    required this.textStyleForItem,
    required this.decorationForItem,
  });

  /// Decoration of the menu container (outside of clip rect)
  final BoxDecoration Function(bool collapsed) menuDecorationOutside;

  /// Decoration of menu container (inside of clip rect)
  final BoxDecoration menuDecorationInside;

  final BoxDecoration menuPreviewDecorationOutside;

  final BoxDecoration menuPreviewDecorationInside;

  final Color Function(bool hasBlur) backgroundTintColor;

  final Color Function(int depth) inactiveMenuVeilColor;

  final Color separatorColor;

  final TextStyle Function(MobileMenuItemInfo) textStyleForItem;
  final BoxDecoration Function(MobileMenuItemInfo) decorationForItem;

  static DefaultMobileMenuTheme themeForBrightness(Brightness brightness) {
    switch (brightness) {
      case Brightness.light:
        return DefaultMobileMenuTheme(
          menuDecorationOutside: (bool collapsed) => BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(collapsed ? 0 : 0.2),
                blurRadius: 10,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          menuDecorationInside: BoxDecoration(
            color: Colors.grey.shade100,
          ),
          menuPreviewDecorationOutside: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.3),
                blurRadius: 12,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          menuPreviewDecorationInside: BoxDecoration(
            color: Colors.white.withOpacity(0.5),
          ),
          backgroundTintColor: (hasBlur) => hasBlur
              ? const Color(0xF3333333).withOpacity(0.3)
              : const Color(0xFF333333).withOpacity(0.5),
          separatorColor: Colors.grey.shade300,
          inactiveMenuVeilColor: (depth) =>
              Colors.grey.shade700.withOpacity((depth * 0.3).clamp(0.0, 0.45)),
          textStyleForItem: (info) => TextStyle(
            color: info.isDestructive
                ? Colors.red
                : info.isDisabled
                    ? Colors.grey
                    : Colors.black,
            fontSize: 15.0,
            decoration: TextDecoration.none,
          ),
          decorationForItem: (info) => BoxDecoration(
            color: info.isPressed ? Colors.grey.shade300 : Colors.transparent,
            border: info.isHeader && !info.isLast
                ? Border(
                    bottom: BorderSide(
                      color: Colors.grey.shade300,
                      width: 1,
                    ),
                  )
                : null,
          ),
        );
      case Brightness.dark:
        return DefaultMobileMenuTheme(
          menuDecorationOutside: (bool collapsed) => BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(collapsed ? 0 : 0.2),
                blurRadius: 15,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          menuDecorationInside: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            border: Border.all(
              color: Colors.white.withOpacity(0.1),
              // width: 10.0,
            ),
            color: const Color(0xFF30323E),
          ),
          menuPreviewDecorationOutside: BoxDecoration(
            borderRadius: BorderRadius.circular(8),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.5),
                blurRadius: 12,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          menuPreviewDecorationInside: BoxDecoration(
            color: const Color(0xFF5B5E75).withOpacity(0.8),
          ),
          backgroundTintColor: (hasBlur) => hasBlur
              ? const Color(0xF3333333).withOpacity(0.3)
              : const Color(0xFF333333).withOpacity(0.5),
          separatorColor: const Color(0xFF4C4F63),
          inactiveMenuVeilColor: (depth) =>
              const ui.Color.fromARGB(255, 35, 36, 45)
                  .withOpacity(((depth * 0.6).clamp(0.0, 0.8))),
          textStyleForItem: (info) => TextStyle(
            color: info.isDestructive
                ? const ui.Color.fromARGB(255, 251, 116, 116)
                : info.isDisabled
                    ? const Color(0xFF636680)
                    : const Color(0xFFC6C5D1),
            fontSize: 15.0,
            decoration: TextDecoration.none,
          ),
          decorationForItem: (info) => BoxDecoration(
            color:
                info.isPressed ? const Color(0xFF4C4F63) : Colors.transparent,
            border: info.isHeader && !info.isLast
                ? const Border(
                    bottom: BorderSide(
                      color: Color(0xFF4C4F63),
                      width: 1,
                    ),
                  )
                : null,
          ),
        );
    }
  }
}

class DefaultMobileMenuWidgetBuilder extends MobileMenuWidgetBuilder {
  DefaultMobileMenuWidgetBuilder({
    bool? enableBackgroundBlur,
    Brightness? brightness,
  }) : _brightness = brightness {
    if (enableBackgroundBlur != null) {
      _enableBackgroundBlur = enableBackgroundBlur;
    } else {
      _checkBackgroundBlur();
    }
  }

  /// Allows overriding brightness for the menu UI.
  final Brightness? _brightness;

  bool _enableBackgroundBlur = false;

  void _checkBackgroundBlur() async {
    if (kIsWeb) {
      _enableBackgroundBlur = false;
    } else if (defaultTargetPlatform == TargetPlatform.android) {
      final deviceInfo = await DeviceInfoPlugin().deviceInfo;
      if (deviceInfo is AndroidDeviceInfo) {
        // There is no straightforward way to determine if Android device is
        // fast enough for background blur so we just enable it for Android 10+,
        // assumption being older devices usually not getting upgrades.
        _enableBackgroundBlur = deviceInfo.version.sdkInt >= 29;
      } else {
        _enableBackgroundBlur = false;
      }
    } else {
      _enableBackgroundBlur = true;
    }
  }

  Brightness _getBrightness(BuildContext context) {
    return _brightness ?? MediaQuery.platformBrightnessOf(context);
  }

  DefaultMobileMenuTheme _getTheme(BuildContext context) {
    return DefaultMobileMenuTheme.themeForBrightness(_getBrightness(context));
  }

  @override
  Widget buildInactiveMenuVeil(
    BuildContext context,
    MobileMenuInfo menuInfo,
  ) {
    final theme = _getTheme(context);
    return AnimatedContainer(
        duration: const Duration(milliseconds: 200),
        color: theme.inactiveMenuVeilColor(menuInfo.depth));
  }

  @override
  Widget buildMenuContainer(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  ) {
    final theme = _getTheme(context);
    final decoration = theme.menuDecorationOutside(menuInfo.isCollapsed);
    final radius = decoration.borderRadius;
    return AnimatedContainer(
      duration: menuInfo.transitionDuration,
      decoration: decoration,
      child: ClipRRect(
        borderRadius: radius ?? BorderRadius.circular(0),
        child: child,
      ),
    );
  }

  @override
  Widget buildMenuContainerInner(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  ) {
    final theme = _getTheme(context);
    return Container(
      decoration: theme.menuDecorationInside,
      child: child,
    );
  }

  @override
  Widget buildMenu(
      BuildContext context, MobileMenuInfo menuInfo, Widget child) {
    return child;
  }

  @override
  Widget buildMenuItemsContainer(
    BuildContext context,
    MobileMenuInfo menuInfo,
    Widget child,
  ) {
    return Scrollbar(child: child);
  }

  @override
  Widget buildMenuHeader(
    BuildContext context,
    MobileMenuInfo menuInfo,
    MobileMenuButtonState state,
  ) {
    return _MenuHeader(
      menuInfo: menuInfo,
      state: state,
      theme: _getTheme(context),
    );
  }

  @override
  Widget buildMenuItem(
    BuildContext context,
    MobileMenuInfo menuInfo,
    MobileMenuButtonState state,
    MenuElement element,
  ) {
    return _MenuItem(
      menuInfo: menuInfo,
      state: state,
      element: element,
      theme: _getTheme(context),
    );
  }

  @override
  Widget buildOverlayBackground(BuildContext context, double opacity) {
    final theme = _getTheme(context);
    if (_enableBackgroundBlur) {
      return Opacity(
        opacity: opacity,
        child: BackdropFilter(
          filter: ui.ImageFilter.blur(
            // sigmaX: 10 * opacity,
            // sigmaY: 10 * opacity,
            sigmaX: 12,
            sigmaY: 12,
          ),
          child: Container(
            color: theme.backgroundTintColor(true),
          ),
        ),
      );
    } else {
      return Opacity(
        opacity: opacity,
        child: Container(
          color: theme.backgroundTintColor(false),
        ),
      );
    }
  }

  @override
  Widget buildMenuPreviewContainer(
    BuildContext context,
    Widget child,
  ) {
    final theme = _getTheme(context);
    final decoration = theme.menuPreviewDecorationOutside;
    return Container(
      decoration: decoration,
      child: ClipRRect(
        borderRadius: decoration.borderRadius ?? BorderRadius.circular(0),
        child: Container(
          decoration: theme.menuPreviewDecorationInside,
          child: child,
        ),
      ),
    );
  }
}

extension on Menu {
  bool hasImage() {
    return children.any(
      (element) =>
          element.image?.asWidget(const IconThemeData.fallback()) != null,
    );
  }
}

class _MenuItem extends StatelessWidget {
  const _MenuItem({
    // ignore: unused_element
    super.key,
    required this.menuInfo,
    required this.state,
    required this.element,
    required this.theme,
  });

  final MobileMenuInfo menuInfo;
  final MobileMenuButtonState state;
  final MenuElement element;
  final DefaultMobileMenuTheme theme;

  @override
  Widget build(BuildContext context) {
    final bool isLast = menuInfo.resolvedChildren.lastOrNull == element;
    Widget? suffix;
    if (element is Menu) {
      suffix = Builder(builder: (context) {
        return _AnimatedChevron(
          isExpanded: false,
          duration: menuInfo.transitionDuration,
          color: DefaultTextStyle.of(context).style.color!,
        );
      });
    } else {
      suffix = null;
    }
    if (element is DeferredMenuElement) {
      return _MenuItemScaffold(
        theme: theme,
        itemInfo: MobileMenuItemInfo(
          isHeader: false,
          isPressed: false,
          isLast: isLast,
          isDestructive: false,
          isDisabled: false,
        ),
        element: element,
        menuInfo: menuInfo,
        child: const Align(
          alignment: Alignment.centerLeft,
          child: SizedBox(
            height: 20,
            width: 20,
            child: CircularProgressIndicator(
              strokeWidth: 2.0,
              color: Colors.grey,
            ),
          ),
        ),
      );
    } else if (element is MenuSeparator) {
      return Container(
        height: 1,
        margin: const EdgeInsets.symmetric(vertical: 4),
        color: theme.separatorColor,
      );
    } else {
      final menuElementAttributes =
          element is MenuAction ? (element as MenuAction).attributes : null;

      return _MenuItemScaffold(
        theme: theme,
        itemInfo: MobileMenuItemInfo(
          isHeader: false,
          isPressed: state.pressed,
          isLast: isLast,
          isDestructive: menuElementAttributes?.destructive ?? false,
          isDisabled: menuElementAttributes?.disabled ?? false,
        ),
        suffix: suffix,
        element: element,
        menuInfo: menuInfo,
        child: Text(
          element.title ?? '',
        ),
      );
    }
  }
}

class _MenuHeader extends StatelessWidget {
  const _MenuHeader({
    // ignore: unused_element
    super.key,
    required this.menuInfo,
    required this.state,
    required this.theme,
  });

  final MobileMenuInfo menuInfo;
  final MobileMenuButtonState state;
  final DefaultMobileMenuTheme theme;

  @override
  Widget build(BuildContext context) {
    Widget? prefix;
    if (menuInfo.menu.image?.asWidget(const IconThemeData.fallback()) == null) {
      final parentPrefixWidth =
          (menuInfo.parentMenu?.hasImage() ?? false) ? 28.0 : 0.0;
      final thisPrefixWidth = menuInfo.menu.hasImage() ? 28.0 : 0.0;

      prefix = AnimatedContainer(
        duration: menuInfo.transitionDuration,
        curve: Curves.easeOutCubic,
        width: menuInfo.isCollapsed ? parentPrefixWidth : thisPrefixWidth,
      );
    }

    return _MenuItemScaffold(
      theme: theme,
      itemInfo: MobileMenuItemInfo(
        isHeader: true,
        isPressed: state.pressed,
        isLast: menuInfo.resolvedChildren.isEmpty,
        isDisabled: false,
        isDestructive: false,
      ),
      element: menuInfo.menu,
      menuInfo: menuInfo,
      // chevron and child need to be builders so that they can access
      // the correct text style set by _MenuItemScaffold.
      suffix: Builder(builder: (context) {
        return _AnimatedChevron(
          isExpanded: !menuInfo.isCollapsed,
          duration: menuInfo.transitionDuration,
          color: DefaultTextStyle.of(context).style.color!,
        );
      }),
      prefix: prefix,
      child: Builder(builder: (context) {
        return AnimatedDefaultTextStyle(
          duration: menuInfo.transitionDuration,
          style: DefaultTextStyle.of(context).style.copyWith(
                fontWeight:
                    menuInfo.isCollapsed ? FontWeight.normal : FontWeight.bold,
              ),
          child: Text(menuInfo.menu.title ?? ''),
        );
      }),
    );
  }
}

class _MenuItemScaffold extends StatelessWidget {
  const _MenuItemScaffold({
    // ignore: unused_element
    super.key,
    this.prefix,
    this.suffix,
    required this.menuInfo,
    required this.element,
    required this.theme,
    required this.itemInfo,
    required this.child,
  });

  final Widget? prefix;
  final Widget? suffix;
  final MobileMenuInfo menuInfo;
  final MenuElement element;
  final DefaultMobileMenuTheme theme;
  final MobileMenuItemInfo itemInfo;
  final Widget child;

  Widget fallbackPrefix(TextStyle textStyle) {
    final iconTheme = menuInfo.iconTheme.copyWith(
      size: 24,
      color: textStyle.color,
    );
    final widget = element.image?.asWidget(iconTheme);
    if (widget != null) {
      return Padding(
        padding: const EdgeInsets.symmetric(horizontal: 2),
        child: widget,
      );
    } else {
      // Will be animated when resolved deferred element introduces image
      return AnimatedContainer(
        curve: Curves.easeOut,
        duration: const Duration(milliseconds: 200),
        width: menuInfo.menu.hasImage() ? 28 : 0,
      );
    }
  }

  Widget? fallbackSuffix(TextStyle style) {
    if (element is MenuAction) {
      final state = (element as MenuAction).state;
      IconData? icon;
      switch (state) {
        case MenuActionState.none:
          break;
        case MenuActionState.checkOn:
          icon = Icons.check;
          break;
        case MenuActionState.checkOff:
          break;
        case MenuActionState.checkMixed:
          icon = Icons.remove;
          break;
        case MenuActionState.radioOn:
          icon = Icons.radio_button_on;
          break;
        case MenuActionState.radioOff:
          icon = Icons.radio_button_off;
          break;
      }
      if (icon != null) {
        return Padding(
          padding: const EdgeInsets.all(2),
          child: Icon(
            icon,
            size: 20,
            color: style.color,
          ),
        );
      }
    }
    return null;
  }

  @override
  Widget build(BuildContext context) {
    final textStyle = theme.textStyleForItem(itemInfo);
    final prefix = this.prefix ?? fallbackPrefix(textStyle);
    final suffix = this.suffix ?? fallbackSuffix(textStyle);

    return DefaultTextStyle(
      style: textStyle,
      child: Container(
          decoration: theme.decorationForItem(itemInfo),
          padding: const EdgeInsets.symmetric(vertical: 7, horizontal: 10),
          child: Row(
            children: [
              prefix,
              Expanded(
                child: Padding(
                  padding:
                      const EdgeInsets.symmetric(vertical: 6, horizontal: 6),
                  child: child,
                ),
              ),
              if (suffix != null) suffix,
            ],
          )),
    );
  }
}

class _AnimatedChevron extends StatelessWidget {
  const _AnimatedChevron({
    Key? key,
    required this.isExpanded,
    required this.duration,
    required this.color,
  }) : super(key: key);

  final bool isExpanded;

  final Duration duration;
  final Color color;

  @override
  Widget build(BuildContext context) {
    final rotation = !isExpanded ? 0.0 : 0.25;
    final offset = !isExpanded ? const Offset(0, 0) : const Offset(0.0, -0.05);
    return AnimatedSlide(
      offset: offset,
      duration: duration,
      curve: Curves.easeOutCubic,
      child: AnimatedRotation(
        duration: duration,
        turns: rotation,
        curve: Curves.easeOutCubic,
        child: Icon(
          Icons.chevron_right_rounded,
          color: color,
          size: 24,
        ),
      ),
    );
  }
}
