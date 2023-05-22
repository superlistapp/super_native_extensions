import 'package:collection/collection.dart';
import 'package:flutter/foundation.dart';
import 'package:flutter/widgets.dart';
import 'package:super_context_menu/src/scaffold/common/deferred_menu_items.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

import '../../menu_model.dart';
import 'menu_layout.dart';
import 'menu_widget_builder.dart';
import 'menu_widget.dart';

class MenuStack extends StatefulWidget {
  final Menu rootMenu;
  final raw.MobileMenuDelegate delegate;
  final MobileMenuWidgetBuilder builder;
  final ValueListenable<bool> canScrollListenable;
  final AlignmentGeometry menuAlignment;
  final IconThemeData iconTheme;

  const MenuStack({
    super.key,
    required this.rootMenu,
    required this.delegate,
    required this.canScrollListenable,
    required this.menuAlignment,
    required this.builder,
    required this.iconTheme,
  });

  @override
  State<StatefulWidget> createState() {
    return MenuStackState();
  }
}

class _MenuRecord {
  _MenuRecord({
    required this.menu,
    required this.sourceRect,
    required this.destinationOffset,
    required this.transition,
  });

  final Menu menu;
  final Rect sourceRect;
  final Offset destinationOffset;
  double transition;
  VoidCallback? onTransitionedToZero;
  final animationKey = GlobalKey();
  final key = GlobalKey<MenuWidgetState>();
}

class MenuStackState extends State<MenuStack> implements MenuWidgetDelegate {
  @override
  void initState() {
    super.initState();
    _records.add(_MenuRecord(
      menu: widget.rootMenu,
      sourceRect: Rect.zero,
      destinationOffset: Offset.zero,
      transition: 1.0,
    ));
  }

  @override
  void pushMenu(BuildContext anchorItem, Menu menu) {
    final existingRecord =
        _beingRemoved.firstWhereOrNull((element) => element.menu == menu);
    if (existingRecord != null) {
      setState(() {
        _beingRemoved.remove(existingRecord);
        _records.add(existingRecord);
        existingRecord.transition = 1.0;
      });
      return;
    }

    final renderObject = anchorItem.findRenderObject() as RenderBox;
    final ourRenderObject = context.findRenderObject() as RenderBox;
    final matrix = renderObject.getTransformTo(ourRenderObject);
    var sourceRect =
        MatrixUtils.transformRect(matrix, renderObject.paintBounds);

    final record = _MenuRecord(
      menu: menu,
      sourceRect: sourceRect,
      destinationOffset: const Offset(0, -4),
      transition: 0.0,
    );
    setState(() {
      _records.add(record);
    });
    WidgetsBinding.instance.addPostFrameCallback((timeStamp) {
      setState(() {
        record.transition = 1.0;
      });
    });
    widget.delegate.didPushSubmenu();
  }

  @override
  void hideMenu({required bool itemSelected}) {
    widget.delegate.hideMenu(itemSelected: itemSelected);
  }

  final _records = <_MenuRecord>[];
  final _beingRemoved = <_MenuRecord>[];

  int _depthForRecord(_MenuRecord record) {
    return _records.length - _records.indexOf(record) - 1;
  }

  MobileMenuInfo _menuInfoForRecord(_MenuRecord record) {
    final allRecords = [
      ..._records,
      ..._beingRemoved,
    ];
    final index = allRecords.indexOf(record);
    final parentMenu = index > 0 ? allRecords[index - 1].menu : null;
    return MobileMenuInfo(
      menu: record.menu,
      resolvedChildren: () => record.key.currentState?.resolvedChildren,
      depth: _records.contains(record) ? _depthForRecord(record) : 0,
      parentMenu: parentMenu,
      isCollapsed: record.transition == 0,
      transitionDuration: const Duration(milliseconds: 250),
      iconTheme: widget.iconTheme,
    );
  }

  @override
  void popMenu() {
    setState(() {
      final last = _records.removeLast();
      _beingRemoved.insert(0, last);
      last.transition = 0;
      last.onTransitionedToZero = () {
        setState(() {
          _beingRemoved.remove(last);
        });
      };
    });
  }

  final _deferredMenuElementCache = DeferredMenuElementCache();

  @override
  Widget build(BuildContext context) {
    final records = [
      ..._records,
      ..._beingRemoved,
    ];
    return MenuLayout(
      children: [
        ...records.map((record) {
          final info = _menuInfoForRecord(record);
          return AnimatedMenuLayoutData(
            key: record.animationKey,
            sourceRect: record.sourceRect,
            destinationOffset: record.destinationOffset,
            transition: record.transition,
            duration: info.transitionDuration,
            curve: record.transition == 1.0
                ? Curves.easeOutCubic
                : Curves.easeInOutCubic,
            onTransitionedToZero: record.onTransitionedToZero,
            child: _MenuContainer(
              builder: widget.builder,
              menuAlignment: widget.menuAlignment,
              onVeilTap: () {
                popMenu();
              },
              info: info,
              offset: 0,
              child: MenuWidget(
                key: record.key,
                builder: widget.builder,
                menuInfo: info,
                delegate: this,
                canScrollListenable: widget.canScrollListenable,
                cache: _deferredMenuElementCache,
              ),
            ),
          );
        }),
      ],
    );
  }
}

class _MenuContainer extends StatelessWidget {
  const _MenuContainer({
    required this.builder,
    required this.info,
    required this.child,
    required this.menuAlignment,
    required this.onVeilTap,
    required this.offset,
  });

  final MobileMenuWidgetBuilder builder;
  final Widget child;
  final MobileMenuInfo info;
  final AlignmentGeometry menuAlignment;
  final double offset;
  final VoidCallback onVeilTap;

  @override
  Widget build(BuildContext context) {
    final menuAlignment = this.menuAlignment.resolve(null);
    final scaleAlignment =
        menuAlignment.x < 0 ? Alignment.topLeft : Alignment.topRight;

    return AnimatedScale(
      duration: info.transitionDuration,
      scale: 1.0 - (info.depth * 0.035).clamp(0, 0.15),
      alignment: scaleAlignment,
      child: builder.buildMenuContainer(
        context,
        info,
        Stack(
          fit: StackFit.passthrough,
          children: [
            builder.buildMenuContainerInner(context, info, child),
            Positioned.fill(
              child: IgnorePointer(
                ignoring: info.depth == 0,
                child: GestureDetector(
                  onTap: onVeilTap,
                  child: builder.buildInactiveMenuVeil(
                    context,
                    info,
                  ),
                ),
              ),
            )
          ],
        ),
      ),
    );
  }
}
