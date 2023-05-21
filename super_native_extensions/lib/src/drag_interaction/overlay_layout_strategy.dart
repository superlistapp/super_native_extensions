import 'dart:math' as math;
import 'package:collection/collection.dart';
import 'package:flutter/widgets.dart';

import 'util.dart';

class MenuLayoutInput {
  MenuLayoutInput({
    required this.layoutMenu,
    required this.bounds,
    required this.primaryItem,
    required this.menuPreviewSize,
    required this.menuDragOffset,
    required this.previousLayoutId,
  });

  final Size Function(BoxConstraints) layoutMenu;
  final Rect bounds;
  final Rect primaryItem;
  final Size menuPreviewSize;
  final double menuDragOffset;
  final String? previousLayoutId;
}

class MenuLayout {
  MenuLayout({
    required this.previewRect,
    required this.menuPosition,
    required this.menuDragExtent,
    required this.canScrollMenu,
    required this.menuAlignment,
    required this.layoutId,
  });

  final String layoutId;
  final Rect previewRect;
  final MenuPosition menuPosition;
  final double menuDragExtent;
  final bool canScrollMenu;
  final AlignmentGeometry menuAlignment;
}

typedef MenuPosition = Offset Function(Rect previewRect);

const _epsilon = 0.001;

class _MenuGeometry {
  final String id;
  final Rect previewRect;
  final Size menuSize;
  final MenuPosition menuPosition;
  final AlignmentGeometry menuAlignment;

  _MenuGeometry({
    required this.id,
    required this.previewRect,
    required this.menuSize,
    required this.menuPosition,
    required this.menuAlignment,
  });

  Rect get menuRect {
    final position = menuPosition(previewRect);
    return Rect.fromLTWH(
      position.dx,
      position.dy,
      menuSize.width,
      menuSize.height,
    );
  }

  Rect get bounds => previewRect.expandToInclude(menuRect);

  bool fitsInto(Rect rect) {
    final bounds = this.bounds;
    return bounds.left + _epsilon >= rect.left &&
        bounds.right <= rect.right + _epsilon &&
        bounds.top + _epsilon >= rect.top &&
        bounds.bottom <= rect.bottom + _epsilon;
  }

  /// Try to keep the menu preview in the same position vertically as before.
  _MenuGeometry _fitIntoHorizontal(Rect rect) {
    final res = _fitInto(rect);
    final desiredPreviewRect = previewRect.moveIntoRect(rect);
    final correction =
        Offset(0, res.previewRect.center.dy - desiredPreviewRect.center.dy);

    return _MenuGeometry(
      id: res.id,
      menuAlignment: res.menuAlignment,
      menuPosition: (pos) => res.menuPosition(pos) + correction,
      menuSize: res.menuSize,
      previewRect: res.previewRect.shift(-correction),
    );
  }

  _MenuGeometry fitInto(Rect rect) {
    if (menuRect.left > previewRect.right ||
        menuRect.right < previewRect.left) {
      return _fitIntoHorizontal(rect);
    } else {
      return _fitInto(rect);
    }
  }

  _MenuGeometry _fitInto(Rect rect) {
    final bounds = this.bounds;
    final dx1 = bounds.left < rect.left ? rect.left - bounds.left : 0.0;
    final dx2 = bounds.right > rect.right ? rect.right - bounds.right : 0.0;
    final dy1 = bounds.top < rect.top ? rect.top - bounds.top : 0.0;
    final dy2 = bounds.bottom > rect.bottom ? rect.bottom - bounds.bottom : 0.0;
    final offset = Offset(dx1 + dx2, dy1 + dy2);
    return _MenuGeometry(
      id: id,
      previewRect: previewRect.shift(offset),
      menuSize: menuSize,
      menuPosition: menuPosition,
      menuAlignment: menuAlignment,
    );
  }
}

/// Picks the geometry that fits best inside the bounds and shifts just enough
/// to fit in the bounds.
_MenuGeometry _bestFitGeometry(
  Rect bounds,
  List<_MenuGeometry> geometry,
  String? previousLayoutId,
) {
  if (previousLayoutId != null) {
    final previous =
        geometry.firstWhereOrNull((element) => element.id == previousLayoutId);
    if (previous != null) {
      return previous.fitInto(bounds);
    }
  }
  // Try to find first element that fully fits
  final firstThatFits =
      geometry.firstWhereOrNull((element) => element.fitsInto(bounds));
  if (firstThatFits != null) {
    return firstThatFits;
  }

  final geometryThatFits = geometry
      .where((element) => element.bounds.size <= bounds.size.inflate(_epsilon))
      .toList(growable: false);

  if (geometryThatFits.isEmpty) {
    return geometry.first;
  }

  // Find which ever geometry needs least adjustment relative to preview rect
  // to fit into bounds
  final best = geometryThatFits.reduce((value, element) {
    final v1 = value.fitInto(bounds);
    final v2 = element.fitInto(bounds);
    final d1 =
        (v1.previewRect.center - value.previewRect.center).distanceSquared;
    final d2 =
        (v2.previewRect.center - element.previewRect.center).distanceSquared;
    return d1 <= d2 + _epsilon ? value : element;
  });
  return best.fitInto(bounds);
}

abstract class MenuLayoutStrategy {
  MenuLayout layout(MenuLayoutInput input);

  static MenuLayoutStrategy forSize(Size screenSize) {
    if (screenSize.shortestSide < 550) {
      // phone layout
      if (screenSize.height > screenSize.width) {
        return _MenuLayoutMobilePortrait();
      } else {
        return _MenuLayout(allowVerticalAttachment: false);
      }
    } else {
      return _MenuLayout(allowVerticalAttachment: true);
    }
  }
}

const _kMenuSpacing = 15.0;

class _MenuLayout extends MenuLayoutStrategy {
  _MenuLayout({
    required this.allowVerticalAttachment,
  });

  final bool allowVerticalAttachment;

  @override
  MenuLayout layout(MenuLayoutInput input) {
    final menuSize = input.layoutMenu(BoxConstraints.loose(input.bounds.size));
    final spaceForPreview = Size(
        input.bounds.width - menuSize.width - _kMenuSpacing,
        input.bounds.height);
    final previewSize = input.menuPreviewSize.fitInto(spaceForPreview);
    final previewRect = Rect.fromCenter(
      center: input.primaryItem.center,
      width: previewSize.width,
      height: previewSize.height,
    );

    final vertical = [
      // Aligned to bottom left corner
      _MenuGeometry(
        id: 'vertical-bottom-left',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.left,
          previewRect.bottom + _kMenuSpacing,
        ),
        menuAlignment: Alignment.topLeft,
      ),
      // Aligned to bottom right corner
      _MenuGeometry(
        id: 'vertical-bottom-right',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.right - menuSize.width,
          previewRect.bottom + _kMenuSpacing,
        ),
        menuAlignment: Alignment.topRight,
      ),
      // Aligned to top left corner
      _MenuGeometry(
        id: 'vertical-top-left',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.left,
          previewRect.top - _kMenuSpacing - menuSize.height,
        ),
        menuAlignment: Alignment.bottomLeft,
      ),
      // Aligned to top right corner
      _MenuGeometry(
        id: 'vertical-top-right',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.right - menuSize.width,
          previewRect.top - _kMenuSpacing - menuSize.height,
        ),
        menuAlignment: Alignment.bottomRight,
      ),
    ];

    final horizontal = [
      // Aligned to top right corner
      _MenuGeometry(
        id: 'horizontal-top-right',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.right + _kMenuSpacing,
          previewRect.top,
        ),
        menuAlignment: Alignment.topLeft,
      ),
      // Aligned to bottom right corner
      _MenuGeometry(
        id: 'horizontal-bottom-right',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.right + _kMenuSpacing,
          previewRect.bottom - menuSize.height,
        ),
        menuAlignment: Alignment.bottomLeft,
      ),
      // Aligned to top left corner
      _MenuGeometry(
        id: 'horizontal-top-left',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.left - _kMenuSpacing - menuSize.width,
          previewRect.top,
        ),
        menuAlignment: Alignment.topRight,
      ),
      // Aligned to bottom left corner
      _MenuGeometry(
        id: 'horizontal-bottom-left',
        previewRect: previewRect,
        menuSize: menuSize,
        menuPosition: (previewRect) => Offset(
          previewRect.left - _kMenuSpacing - menuSize.width,
          previewRect.bottom - menuSize.height,
        ),
        menuAlignment: Alignment.bottomRight,
      ),
    ];

    final List<_MenuGeometry> geometries;
    if (allowVerticalAttachment &&
        input.menuPreviewSize.width > input.menuPreviewSize.height) {
      // prefer vertical attachment on wide previews
      geometries = [
        ...vertical,
        ...horizontal,
      ];
    } else if (allowVerticalAttachment) {
      // prefer horizontal attachment on wide previews
      geometries = [
        ...horizontal,
        ...vertical,
      ];
    } else {
      geometries = horizontal;
    }

    final geometry = _bestFitGeometry(
      input.bounds,
      geometries,
      input.previousLayoutId,
    );

    return MenuLayout(
      layoutId: geometry.id,
      previewRect: geometry.previewRect,
      menuPosition: geometry.menuPosition,
      menuDragExtent: 0.0,
      canScrollMenu: true,
      menuAlignment: geometry.menuAlignment,
    );
  }
}

class _MenuLayoutMobilePortrait extends MenuLayoutStrategy {
  @override
  MenuLayout layout(MenuLayoutInput input) {
    final menuPreviewSizeMin = input.menuPreviewSize
        .fitInto(Size(input.bounds.width, input.bounds.height / 4));
    final menuPreviewSizeMax = input.menuPreviewSize
        .fitInto(Size(input.bounds.width, input.bounds.height * 3 / 4));

    final menuSize = input.layoutMenu(BoxConstraints.loose(Size(
      input.bounds.width,
      input.bounds.height - menuPreviewSizeMin.height - _kMenuSpacing,
    )));

    final menuOverflow = math.max(
        menuPreviewSizeMax.height +
            _kMenuSpacing +
            menuSize.height -
            input.bounds.height,
        0.0);

    final verticalCorrection = -math.max(
        input.primaryItem.center.dy -
            input.bounds.top +
            menuPreviewSizeMin.height / 2 +
            _kMenuSpacing +
            menuSize.height -
            input.bounds.height,
        0.0);

    final menuDragOffset = input.menuDragOffset * menuOverflow;

    final actualMenuPreviewSize = input.menuPreviewSize.fitInto(
        Size(input.bounds.width, menuPreviewSizeMax.height - menuDragOffset));

    final menuPreviewRectMax = Rect.fromCenter(
            center: input.primaryItem.center,
            width: menuPreviewSizeMax.width,
            height: menuPreviewSizeMax.height)
        .translate(0, verticalCorrection)
        .fitIntoRect(input.bounds);

    // left aligned
    final previewRect1 = menuPreviewRectMax.copyWith(
      width: actualMenuPreviewSize.width,
      height: actualMenuPreviewSize.height,
    );

    // right aligned
    final previewRect2 = previewRect1.copyWith(
      left: menuPreviewRectMax.right - previewRect1.width,
    );

    // Bounds adjusted to fit overflow in so that _bestFitGeomety doesn't try to move things
    // vertically
    final adjustedBounds =
        input.bounds.copyWith(height: input.bounds.height + menuOverflow);
    final geometry = _bestFitGeometry(
      adjustedBounds,
      [
        _MenuGeometry(
          id: 'geometry-1',
          previewRect: previewRect1,
          menuSize: menuSize,
          menuPosition: (previewRect) => Offset(
            previewRect.left,
            previewRect.bottom + _kMenuSpacing,
          ),
          menuAlignment: Alignment.topLeft,
        ),
        _MenuGeometry(
          id: 'geometry-2',
          previewRect: previewRect2,
          menuSize: menuSize,
          menuPosition: (previewRect) => Offset(
            previewRect.right - menuSize.width,
            previewRect.bottom + _kMenuSpacing,
          ),
          menuAlignment: Alignment.topRight,
        ),
      ],
      input.previousLayoutId,
    );

    return MenuLayout(
      layoutId: geometry.id,
      previewRect: geometry.previewRect,
      menuDragExtent: menuOverflow,
      canScrollMenu: menuOverflow == 0.0 || input.menuDragOffset == 1.0,
      menuPosition: geometry.menuPosition,
      menuAlignment: geometry.menuAlignment,
    );
  }
}
