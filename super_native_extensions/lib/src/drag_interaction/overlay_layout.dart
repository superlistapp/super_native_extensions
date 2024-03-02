import 'dart:ui' as ui;
import 'dart:math' as math;

import 'package:flutter/rendering.dart';
import 'package:pixel_snap/pixel_snap.dart';

import '../widget_snapshot/widget_snapshot.dart';
import 'drag_state_machine.dart';
import 'overlay_layout_strategy.dart';
import 'util.dart';

class LayoutItemConfiguration {
  LayoutItemConfiguration({
    required this.index,
    required this.liftChildId,
    required this.dragChildId,
    required this.liftRect,
    required this.dragSize,
    required this.liftImage,
    required this.dragImage,
  });

  final int index;
  final Object liftChildId;
  final Object dragChildId;
  final Rect liftRect;
  final Size dragSize;
  final WidgetSnapshot liftImage;
  final WidgetSnapshot dragImage;
}

class OverlayLayoutState {
  String? _previousLayoutId;
}

class OverlayLayoutDelegate extends MultiChildLayoutDelegate {
  OverlayLayoutDelegate({
    required this.padding,
    required this.primaryItem,
    required this.secondaryItems,
    required this.menuPreviewSize,
    required this.menuPreviewId,
    required this.menuId,
    required this.dragState,
    required this.menuDragExtentSetter,
    required this.canScrollMenuSetter,
    required this.menuAlignmentSetter,
    required this.pixelSnap,
    required this.hasCustomMenuPreview,
    required this.layoutState,
  });

  final EdgeInsets padding;
  final LayoutItemConfiguration primaryItem;
  final List<LayoutItemConfiguration> secondaryItems;
  final Size? menuPreviewSize;
  final Object menuPreviewId;
  final Object menuId;
  final DragState dragState;
  final ValueSetter<double> menuDragExtentSetter;
  final ValueSetter<bool> canScrollMenuSetter;
  final ValueSetter<AlignmentGeometry> menuAlignmentSetter;
  final PixelSnap pixelSnap;
  final bool hasCustomMenuPreview;
  final OverlayLayoutState layoutState;

  double _dampen(double value) {
    return math.log(value * value + 1) / math.ln10;
  }

  Offset get _menuOverdrag {
    return Offset(
      _dampen(dragState.menuOverdrag.dx.abs()) * dragState.menuOverdrag.dx.sign,
      _dampen(dragState.menuOverdrag.dy.abs()) * dragState.menuOverdrag.dy.sign,
    );
  }

  double _inflateFactorForSize(Size size) {
    final ratio = math
        .max(primaryItem.liftRect.width / size.width,
            primaryItem.liftRect.height / size.height)
        .clamp(0.0, 1.0);
    const maxFactor = 1.15;
    const minFactor = 1.04;
    return ui.lerpDouble(maxFactor, minFactor, ratio)!;
  }

  Rect _computePrimaryItemRect({
    required Rect bounds,
    required Rect? menuPreviewRect,
    required double liftFactor,
  }) {
    final inflateFactor = 1.0 + (liftFactor - 1.0) * dragState.liftFactor;
    var rect = primaryItem.liftRect.inflateBy(inflateFactor);

    // If there is no custom menu preview use the size from lift rect.
    // This is similar behavior to iOS.
    if (!hasCustomMenuPreview && menuPreviewRect != null) {
      menuPreviewRect =
          menuPreviewRect.inflateBy(inflateFactor).fitIntoRect(bounds);
    }

    if (hasChild(menuPreviewId) && dragState.menuFactor > 0) {
      assert(menuPreviewRect != null);
      final menuOverdrag = _menuOverdrag;
      final menuRect =
          menuPreviewRect!.translate(menuOverdrag.dx, menuOverdrag.dy);
      rect = Rect.lerp(rect, menuRect, dragState.menuFactor)!;
    }
    if (dragState.dragFactor > 0) {
      final finalDragRect = Rect.fromCenter(
        center: dragState.globalPosition,
        width: primaryItem.dragSize.width,
        height: primaryItem.dragSize.height,
      );
      rect = Rect.lerp(rect, finalDragRect, dragState.dragFactor)!;
    }
    return rect;
  }

  Rect _computeSecondaryItemRect(LayoutItemConfiguration item) {
    var rect = item.liftRect;
    if (dragState.dragFactor > 0) {
      final finalDragRect = Rect.fromCenter(
        center: dragState.globalPosition,
        width: item.dragSize.width,
        height: item.dragSize.height,
      );
      rect = Rect.lerp(rect, finalDragRect, dragState.dragFactor)!;
    }
    return rect;
  }

  void _layoutChild(Object childId, Rect rect) {
    layoutChild(childId, BoxConstraints.tight(rect.size));
    positionChild(childId, rect.topLeft.pixelSnap(pixelSnap));
  }

  EdgeInsets _insetsForSize(ui.Size size) {
    return padding + const EdgeInsets.all(20);
  }

  @override
  void performLayout(ui.Size size) {
    MenuLayoutStrategy? strategy;
    Rect? menuPreviewRect;
    final insets = _insetsForSize(size);
    final bounds =
        insets.deflateRect(Rect.fromLTWH(0, 0, size.width, size.height));
    Size? menuSize;
    MenuPosition? menuPosition;
    if (hasChild(menuPreviewId)) {
      strategy = MenuLayoutStrategy.forSize(size);
      final layout = strategy.layout(
        MenuLayoutInput(
            previousLayoutId: layoutState._previousLayoutId,
            layoutMenu: (constraints) {
              menuSize = layoutChild(menuId, constraints);
              return menuSize!;
            },
            bounds: bounds,
            primaryItem: primaryItem.liftRect,
            menuPreviewSize: menuPreviewSize!,
            menuDragOffset: dragState.menuDragOffset),
      );
      menuDragExtentSetter(layout.menuDragExtent);
      canScrollMenuSetter(layout.canScrollMenu);
      menuAlignmentSetter(layout.menuAlignment);
      menuPreviewRect = layout.previewRect;
      menuPosition = layout.menuPosition;
      layoutState._previousLayoutId = layout.layoutId;
    }

    final primaryItemRect = _computePrimaryItemRect(
      bounds: bounds,
      menuPreviewRect: menuPreviewRect,
      liftFactor: _inflateFactorForSize(size),
    );
    _layoutChild(primaryItem.liftChildId, primaryItemRect);
    _layoutChild(primaryItem.dragChildId, primaryItemRect);
    if (hasChild(menuPreviewId)) {
      _layoutChild(menuPreviewId, primaryItemRect);
    }

    for (final item in secondaryItems) {
      final rect = _computeSecondaryItemRect(item);
      _layoutChild(item.liftChildId, rect);
      _layoutChild(item.dragChildId, rect);
    }

    if (hasChild(menuId)) {
      assert(menuPosition != null);
      final offset = menuPosition!(primaryItemRect) - _menuOverdrag;
      positionChild(menuId, offset.pixelSnap(pixelSnap));
    }
  }

  @override
  bool shouldRelayout(covariant MultiChildLayoutDelegate oldDelegate) {
    return this != oldDelegate;
  }
}
