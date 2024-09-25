import 'dart:async';

import 'package:flutter/foundation.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import '../image_data.dart';
import '../menu_image_impl.dart';
import '../menu.dart';
import '../menu_model.dart';
import '../cancellation_token.dart';
import '../util.dart';
import '../widget_snapshot/widget_snapshot.dart';
import 'context.dart';
import 'image_data.dart';

Future<dynamic> _serializeImage(
    MenuImage? image, MenuSerializationOptions options,
    {required bool destructive}) async {
  if (image is SystemMenuImage) {
    return {
      'type': 'system',
      'name': image.systemImageName,
    };
  } else if (image != null) {
    final iconTheme =
        destructive ? options.destructiveIconTheme : options.iconTheme;
    final i = await image.asImage(iconTheme, options.devicePixelRatio);
    if (i != null) {
      i.devicePixelRatio ??= options.devicePixelRatio;
      final res = {
        'type': 'image',
        'data': (await ImageData.fromImage(i)).serialize(),
      };
      i.dispose();
      return res;
    }
  }
  return null;
}

extension on Menu {
  Future<dynamic> serialize(MenuSerializationOptions options) async => {
        'type': 'menu',
        'content': {
          'uniqueId': uniqueId,
          'title': title,
          'subtitle': subtitle,
          'image': await _serializeImage(image, options, destructive: false),
          'children': await Future.wait(
            children.map(
              (e) => e.serialize(options),
            ),
          ),
        }
      };
}

extension on MenuActionAttributes {
  Future<dynamic> serialize() async => {
        'disabled': disabled,
        'destructive': destructive,
      };
}

extension on MenuSeparator {
  Future<dynamic> serialize() async => {
        'type': 'separator',
        'content': {
          'title': title,
        }
      };
}

extension on SingleActivator {
  dynamic serialize() => {
        'trigger': trigger.keyLabel,
        'alt': alt,
        'meta': meta,
        'shift': shift,
        'control': control,
      };
}

extension on MenuAction {
  Future<dynamic> serialize(MenuSerializationOptions options) async => {
        'type': 'action',
        'content': {
          'uniqueId': uniqueId,
          'title': title,
          'subtitle': subtitle,
          'image': await _serializeImage(image, options,
              destructive: attributes.destructive),
          'attributes': await attributes.serialize(),
          'state': state.name,
          'activator': activator?.serialize(),
        }
      };
}

extension on DeferredMenuElement {
  Future<dynamic> serialize() async => {
        'type': 'deferred',
        'content': {
          'uniqueId': uniqueId,
        }
      };
}

extension on MenuElement {
  Future<dynamic> serialize(MenuSerializationOptions options) {
    if (this is Menu) {
      return (this as Menu).serialize(options);
    } else if (this is MenuAction) {
      return (this as MenuAction).serialize(options);
    } else if (this is DeferredMenuElement) {
      return (this as DeferredMenuElement).serialize();
    } else if (this is MenuSeparator) {
      return (this as MenuSeparator).serialize();
    } else {
      throw Exception('Unknown menu element type');
    }
  }
}

final _channel =
    NativeMethodChannel('MenuManager', context: superNativeExtensionsContext);

class NativeMenuHandle extends MenuHandle {
  NativeMenuHandle({
    required this.handle,
    required this.menu,
    required this.elements,
    required this.serializationOptions,
  });

  @override
  final Menu menu;

  final int handle;
  final List<MenuElement> elements;

  final MenuSerializationOptions serializationOptions;

  void onDispose(VoidCallback callback) {
    _onDispose.add(callback);
  }

  final _onDispose = <VoidCallback>[];

  @override
  void dispose() {
    for (final c in _onDispose) {
      c();
    }
    _onDispose.clear();
  }
}

class _ElementWithHandle {
  _ElementWithHandle({
    required this.element,
    required this.handle,
  });

  final MenuElement element;
  final NativeMenuHandle handle;
}

class MenuContextImpl extends MenuContext {
  final _handles = <NativeMenuHandle>[];

  @override
  Future<void> initialize() async {
    super.initialize();
    final engineHandle = await EngineContext.instance.getEngineHandle();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod('newContext', {
      'engineHandle': engineHandle,
    });
  }

  _ElementWithHandle? _elementWithId(int uniqueId) {
    for (final handle in _handles) {
      for (final element in handle.elements) {
        final e = element.find(uniqueId: uniqueId);
        if (e != null) {
          return _ElementWithHandle(element: e, handle: handle);
        }
      }
    }
    return null;
  }

  double _platformIconSize() {
    if (defaultTargetPlatform == TargetPlatform.macOS ||
        defaultTargetPlatform == TargetPlatform.linux) {
      return 16;
    } else {
      return 24;
    }
  }

  @override
  Future<MenuResult> showContextMenu(DesktopContextMenuRequest request) async {
    final res = await _channel.invokeMethod('showContextMenu', {
      'menuHandle': (request.menu as NativeMenuHandle).handle,
      'location': request.position.serialize(),
      if (request.writingToolsConfiguration != null)
        'writingToolsConfiguration':
            request.writingToolsConfiguration!.serialize(),
    }) as Map;
    return MenuResult(
      itemSelected: res['itemSelected'],
    );
  }

  @override
  Future<MenuHandle> registerMenu(
    Menu menu,
    // ignore: avoid_renaming_method_parameters, no_leading_underscores_for_local_identifiers
    MenuSerializationOptions _options,
  ) async {
    final platformIconSize = _platformIconSize();
    final options = MenuSerializationOptions(
      iconTheme: _options.iconTheme.copyWith(size: platformIconSize),
      destructiveIconTheme: _options.destructiveIconTheme.copyWith(
        size: platformIconSize,
      ),
      devicePixelRatio: _options.devicePixelRatio,
    );
    // The cast is necessary for correct extension method to be called.
    // ignore: unnecessary_cast
    final serialized = await (menu as MenuElement).serialize(options);
    final handle =
        await _channel.invokeMethod('registerMenu', serialized) as int;
    final res = NativeMenuHandle(
      menu: menu,
      elements: [menu],
      handle: handle,
      serializationOptions: options,
    );
    res.onDispose(() {
      _handles.removeWhere((element) => element.handle == handle);
      _channel.invokeMethod('disposeMenu', handle);
    });
    _handles.add(res);
    return res;
  }

  void _updatePreviewImage(
      int configurationId, WidgetSnapshot previewImage) async {
    final imageData = await ImageData.fromImage(previewImage.image);
    _channel.invokeMethod('updatePreviewImage', {
      'configurationId': configurationId,
      'image': imageData.serialize(),
    });
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'getConfigurationForLocation') {
      return handleError(() async {
        final arguments = call.arguments as Map;
        final offset = OffsetExt.deserialize(arguments['location']);
        final configurationId = arguments['configurationId'] as int;
        final configuration = await delegate?.getMenuConfiguration(
          MobileMenuConfigurationRequest(
              configurationId: configurationId,
              location: offset,
              previewImageSetter: (image) =>
                  _updatePreviewImage(configurationId, image)),
        );
        if (configuration != null) {
          return {'configuration': await configuration.serialize()};
        } else {
          return {'configuration': null};
        }
      }, () => {'configuration': null});
    } else if (call.method == 'onAction') {
      return handleError(() async {
        final actionId = call.arguments as int;
        final element = _elementWithId(actionId)?.element;
        if (element is MenuAction) {
          element.callback();
        }
      }, () => null);
    } else if (call.method == 'onShowMenu') {
      return handleError(() async {
        delegate?.onShowMenu(call.arguments as int);
      }, () => null);
    } else if (call.method == 'onHideMenu') {
      return handleError(() async {
        final hideRequest = call.arguments as Map;
        delegate?.onHideMenu(
            hideRequest['menuConfigurationId'] as int,
            MenuResult(
              itemSelected: hideRequest['itemSelected'] as bool,
            ));
      }, () => null);
    } else if (call.method == 'onPreviewAction') {
      return handleError(() async {
        delegate?.onPreviewAction(call.arguments as int);
      }, () => null);
    } else if (call.method == 'getDeferredMenu') {
      return handleError(() async {
        final id = call.arguments as int;
        final element = _elementWithId(id);
        Iterable<dynamic> res = [];
        if (element != null && element.element is DeferredMenuElement) {
          final menu = await getDeferredMenu(
            element.handle,
            element.element as DeferredMenuElement,
          );
          res = await Future.wait(menu.map((e) {
            element.handle.elements.add(e);
            return e.serialize(element.handle.serializationOptions);
          }));
        }
        return {'elements': res};
      }, () => {'elements': []});
    } else if (call.method == 'sendWritingToolsReplacement') {
      final text = call.arguments['text'] as String;
      writingToolsSuggestionCallback?.call(text);
    } else {
      return null;
    }
  }

  Future<List<MenuElement>> getDeferredMenu(
    MenuHandle handle,
    DeferredMenuElement element,
  ) async {
    final completer = Completer<List<MenuElement>>();
    final token = SimpleCancellationToken();
    element.provider(token).then((value) {
      if (!token.cancelled) {
        token.dispose();
        completer.complete(value);
      }
    }, onError: (e) {
      if (!token.cancelled) {
        token.dispose();
        completer.completeError(e);
      }
    });
    (handle as NativeMenuHandle).onDispose(() {
      if (!completer.isCompleted) {
        completer.complete([]);
        token.cancel();
      }
    });
    return completer.future;
  }
}

extension MenuConfigurationExt on MobileMenuConfiguration {
  Future<dynamic> serialize() async => {
        'configurationId': configurationId,
        'previewImage': previewImage != null
            ? (await ImageData.fromImage(previewImage!.image)).serialize()
            : null,
        'previewSize': previewSize?.serialize(),
        'liftImage': (await liftImage.intoRaw()).serialize(),
        'menuHandle': (handle as NativeMenuHandle).handle,
      };
}
