import 'package:flutter/foundation.dart';
import 'package:flutter/material.dart'
    show Colors, Icons, CircularProgressIndicator;
import 'package:pixel_snap/widgets.dart';
import 'package:super_context_menu/src/default_builder/group_intrinsic_width.dart';
import 'package:super_native_extensions/raw_menu.dart';

import '../menu_model.dart';
import '../scaffold/desktop/menu_widget_builder.dart';

class DesktopMenuItemInfo {
  final bool disabled;
  final bool selected;
  final bool destructive;
  final bool menuFocused;

  DesktopMenuItemInfo({
    required this.disabled,
    required this.destructive,
    required this.selected,
    required this.menuFocused,
  });
}

extension on SingleActivator {
  String stringRepresentation() {
    return [
      if (control) 'Ctrl',
      if (alt) 'Alt',
      if (meta) defaultTargetPlatform == TargetPlatform.macOS ? 'Cmd' : 'Meta',
      if (shift) 'Shift',
      trigger.keyLabel,
    ].join('+');
  }
}

class DefaultDesktopMenuTheme {
  final BoxDecoration decorationOuter; // Outside of clip
  final BoxDecoration decorationInner; // Inside of clip
  final Color separatorColor;
  final TextStyle Function(DesktopMenuItemInfo) textStyleForItem;
  final TextStyle Function(DesktopMenuItemInfo, TextStyle)
      textStyleForItemActivator;
  final BoxDecoration Function(DesktopMenuItemInfo) decorationForItem;

  DefaultDesktopMenuTheme({
    required this.decorationOuter,
    required this.decorationInner,
    required this.separatorColor,
    required this.textStyleForItem,
    required this.textStyleForItemActivator,
    required this.decorationForItem,
  });

  static DefaultDesktopMenuTheme themeForBrightness(Brightness brightness) {
    switch (brightness) {
      case Brightness.dark:
        return DefaultDesktopMenuTheme(
          decorationOuter: BoxDecoration(
            color: Colors.grey.shade900,
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.5),
                blurRadius: 10,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          decorationInner: BoxDecoration(
            border: Border.all(
              color: Colors.grey.shade700,
              width: 1,
            ),
            borderRadius: BorderRadius.circular(6),
            color: Colors.grey.shade800,
          ),
          separatorColor: Colors.grey.shade600,
          textStyleForItem: (info) {
            Color color;
            if (info.selected && info.menuFocused) {
              color = Colors.white;
            } else if (info.destructive) {
              color = const Color(0xFFFF4500);
            } else if (info.disabled) {
              color = Colors.grey;
            } else {
              color = Colors.white;
            }
            return TextStyle(
              color: color,
              fontSize: 14.0,
              decoration: TextDecoration.none,
            );
          },
          textStyleForItemActivator: (info, textStyle) {
            return textStyle.copyWith(
              fontSize: 12.5,
              color: textStyle.color!.withOpacity(0.5),
            );
          },
          decorationForItem: (info) {
            Color color;
            if (info.selected && info.menuFocused) {
              color = Colors.blue.shade600;
            } else if (info.selected) {
              color = Colors.blue.withOpacity(0.3);
            } else {
              color = Colors.transparent;
            }
            return BoxDecoration(
              color: color,
              borderRadius: BorderRadius.circular(4.0),
            );
          },
        );
      case Brightness.light:
        return DefaultDesktopMenuTheme(
          decorationOuter: BoxDecoration(
            color: Colors.black.withOpacity(0.2),
            boxShadow: [
              BoxShadow(
                color: Colors.black.withOpacity(0.25),
                blurRadius: 12,
                spreadRadius: 0,
                offset: const Offset(0, 0),
              ),
            ],
          ),
          decorationInner: BoxDecoration(
            borderRadius: BorderRadius.circular(6.0),
            color: Colors.white,
          ),
          separatorColor: Colors.grey.shade300,
          textStyleForItem: (info) {
            Color color;
            if (info.selected && info.menuFocused) {
              color = Colors.white;
            } else if (info.destructive) {
              color = Colors.red;
            } else if (info.disabled) {
              color = Colors.grey;
            } else {
              color = Colors.black;
            }
            return TextStyle(
              color: color,
              fontSize: 14.0,
              decoration: TextDecoration.none,
            );
          },
          textStyleForItemActivator: (info, textStyle) {
            return textStyle.copyWith(
              fontSize: 12.5,
              color: textStyle.color!.withOpacity(0.5),
            );
          },
          decorationForItem: (info) {
            Color color;
            if (info.selected && info.menuFocused) {
              color = Colors.blue;
            } else if (info.selected) {
              color = Colors.blue.withOpacity(0.3);
            } else {
              color = Colors.transparent;
            }
            return BoxDecoration(
              color: color,
              borderRadius: BorderRadius.circular(4.0),
            );
          },
        );
    }
  }
}

class DefaultDesktopMenuWidgetBuilder extends DesktopMenuWidgetBuilder {
  DefaultDesktopMenuWidgetBuilder({
    this.maxWidth = 450,
  });

  final double maxWidth;

  static DefaultDesktopMenuTheme _themeForContext(BuildContext context) {
    return DefaultDesktopMenuTheme.themeForBrightness(
        MediaQuery.platformBrightnessOf(context));
  }

  @override
  Widget buildMenuContainer(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Widget child,
  ) {
    final pixelRatio = MediaQuery.of(context).devicePixelRatio;
    final theme = _themeForContext(context);
    return Container(
      decoration: theme.decorationOuter.copyWith(
          borderRadius: BorderRadius.circular(6.0 + 1.0 / pixelRatio)),
      child: ClipRRect(
        borderRadius: BorderRadius.circular(6),
        child: Padding(
          padding: EdgeInsets.all(1.0 / pixelRatio),
          child: Container(
            decoration: theme.decorationInner,
            padding: const EdgeInsets.symmetric(vertical: 6.0),
            child: DefaultTextStyle(
              style: const TextStyle(
                color: Colors.black,
                fontSize: 14.0,
                decoration: TextDecoration.none,
              ),
              child: ConstrainedBox(
                constraints: BoxConstraints(maxWidth: maxWidth),
                child: GroupIntrinsicWidthContainer(child: child),
              ),
            ),
          ),
        ),
      ),
    );
  }

  @override
  Widget buildSeparator(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    MenuSeparator separator,
  ) {
    final theme = _themeForContext(context);
    final paddingLeft = 10.0 + (menuInfo.hasAnyCheckedItems ? (16 + 6) : 0);
    const paddingRight = 10.0;
    return Container(
      height: 1,
      margin: EdgeInsets.only(
        left: paddingLeft,
        right: paddingRight,
        top: 5,
        bottom: 6,
      ),
      color: theme.separatorColor,
    );
  }

  IconData? _stateToIcon(MenuActionState state) {
    switch (state) {
      case MenuActionState.none:
        return null;
      case MenuActionState.checkOn:
        return Icons.check;
      case MenuActionState.checkOff:
        return null;
      case MenuActionState.checkMixed:
        return Icons.remove;
      case MenuActionState.radioOn:
        return Icons.radio_button_on;
      case MenuActionState.radioOff:
        return Icons.radio_button_off;
    }
  }

  @override
  Widget buildMenuItem(
    BuildContext context,
    DesktopMenuInfo menuInfo,
    Key innerKey,
    DesktopMenuButtonState state,
    MenuElement element,
  ) {
    final theme = _themeForContext(context);
    final itemInfo = DesktopMenuItemInfo(
      destructive: element is MenuAction && element.attributes.destructive,
      disabled: element is MenuAction && element.attributes.disabled,
      menuFocused: menuInfo.focused,
      selected: state.selected,
    );
    final textStyle = theme.textStyleForItem(itemInfo);
    final iconTheme = menuInfo.iconTheme.copyWith(
      size: 16,
      color: textStyle.color,
    );
    final stateIcon =
        element is MenuAction ? _stateToIcon(element.state) : null;
    final Widget? prefix;
    if (stateIcon != null) {
      prefix = Icon(
        stateIcon,
        size: 16,
        color: iconTheme.color,
      );
    } else if (menuInfo.hasAnyCheckedItems) {
      prefix = const SizedBox(width: 16);
    } else {
      prefix = null;
    }
    final image = element.image?.asWidget(iconTheme);

    final Widget? suffix;
    if (element is Menu) {
      suffix = Icon(
        Icons.chevron_right_outlined,
        size: 18,
        color: iconTheme.color,
      );
    } else if (element is MenuAction) {
      final activator = element.activator?.stringRepresentation();
      if (activator != null) {
        suffix = Padding(
          padding: const EdgeInsetsDirectional.only(end: 6),
          child: Text(
            activator,
            style: theme.textStyleForItemActivator(itemInfo, textStyle),
          ),
        );
      } else {
        suffix = null;
      }
    } else {
      suffix = null;
    }

    final child = element is DeferredMenuElement
        ? const Align(
            alignment: Alignment.centerLeft,
            child: SizedBox(
              height: 16,
              width: 16,
              child: CircularProgressIndicator(
                strokeWidth: 2.0,
                color: Colors.grey,
              ),
            ),
          )
        : Text(
            maxLines: 1,
            overflow: TextOverflow.ellipsis,
            element.title ?? '',
            style: textStyle,
          );

    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 6.0),
      child: Container(
        key: innerKey,
        padding: const EdgeInsets.all(5),
        decoration: theme.decorationForItem(itemInfo),
        child: Row(
          children: [
            if (prefix != null) prefix,
            if (prefix != null) const SizedBox(width: 6.0),
            if (image != null) image,
            if (image != null) const SizedBox(width: 4.0),
            Expanded(
              child: Padding(
                padding: const EdgeInsets.symmetric(horizontal: 2.0),
                child: child,
              ),
            ),
            GroupIntrinsicWidth(
              child: Container(
                child: Row(
                  mainAxisSize: MainAxisSize.max,
                  mainAxisAlignment: MainAxisAlignment.end,
                  children: [
                    if (suffix != null) const SizedBox(width: 6.0),
                    if (suffix != null) suffix,
                  ],
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}

extension on DesktopMenuInfo {
  bool get hasAnyCheckedItems => (resolvedChildren.any((element) =>
      element is MenuAction && element.state != MenuActionState.none));
}
