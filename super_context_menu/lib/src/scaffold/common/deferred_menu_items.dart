import 'package:flutter/widgets.dart';
import 'package:super_native_extensions/raw_menu.dart' as raw;

import '../../menu_model.dart';

class DeferredMenuElementCache {
  final _cache = <int, List<MenuElement>>{};
}

mixin DeferredMenuItemsContainer<ChildType, WidgetType extends StatefulWidget>
    on State<WidgetType> {
  final _inProgressTokens = <raw.SimpleCancellationToken>[];

  late List<ChildType> resolvedChildren;

  late DeferredMenuElementCache _cache;

  ChildType newChild(MenuElement e);

  bool childHasMenuElement(ChildType element, MenuElement menuElement);

  void initDeferredElements(
    List<MenuElement> initialChildren,
    DeferredMenuElementCache cache,
  ) {
    _cache = cache;
    resolvedChildren = initialChildren.map(newChild).toList();
    _loadDeferred(initialChildren);
  }

  @override
  void dispose() {
    super.dispose();
    for (final token in _inProgressTokens) {
      token.cancel();
    }
  }

  void _loadDeferred(List<MenuElement> initialChildren) {
    for (final element in initialChildren) {
      if (element is DeferredMenuElement) {
        _loadDeferredElement(element);
      }
    }
  }

  void _loadDeferredElement(DeferredMenuElement element) {
    final cached = _cache._cache[element.uniqueId];
    if (cached != null) {
      _didLoadItemsForElement(element, cached);
      return;
    }

    final token = raw.SimpleCancellationToken();
    element.provider(token).then((value) {
      if (!token.cancelled) {
        token.dispose();
      }
      _inProgressTokens.remove(token);
      _cache._cache[element.uniqueId] = value;
      if (mounted) {
        _didLoadItemsForElement(element, value);
      }
    });
    _inProgressTokens.add(token);
  }

  void _didLoadItemsForElement(
      DeferredMenuElement element, List<MenuElement> items) {
    final index =
        resolvedChildren.indexWhere((e) => childHasMenuElement(e, element));
    if (index != -1) {
      setState(() {
        resolvedChildren.removeAt(index);
        resolvedChildren.insertAll(index, items.map(newChild));
      });
    }
  }
}
