import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';

import '../../raw_drag_drop.dart';
import '../default_menu_image.dart';
import '../menu.dart';
import '../cancellation_token.dart';
import '../util.dart';
import 'api_model.dart';
import 'context.dart';

Future<dynamic> _serializeImage(
    MenuImage? image, MenuSerializationOptions options) async {
  if (image is SystemMenuImage) {
    return {
      'type': 'system',
      'name': image.systemImageName,
    };
  } else if (image != null) {
    final i = await image.asImage(options.iconTheme, options.devicePixelRatio);
    if (i != null) {
      i.devicePixelRatio ??= options.devicePixelRatio;
      return {
        'type': 'image',
        'data': (await ImageData.fromImage(i)).serialize(),
      };
    }
  }
  return null;
}

extension on Menu {
  Future<dynamic> serialize(MenuSerializationOptions options) async => {
        'type': 'menu',
        'content': {
          'uniqueId': uniqueId,
          'identifier': identifier,
          'title': title,
          'subtitle': subtitle,
          'image': await _serializeImage(image, options),
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

extension on Separator {
  Future<dynamic> serialize() async => {
        'type': 'separator',
        'content': {
          'title': title,
        }
      };
}

extension on MenuAction {
  Future<dynamic> serialize(MenuSerializationOptions options) async => {
        'type': 'action',
        'content': {
          'uniqueId': uniqueId,
          'identifier': identifier,
          'title': title,
          'subtitle': subtitle,
          'image': await _serializeImage(image, options),
          'attributes': await attributes.serialize(),
          'state': state.name,
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
    } else if (this is Separator) {
      return (this as Separator).serialize();
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
    WidgetsFlutterBinding.ensureInitialized();
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
    return 24; // TODO(knopp): Per platform
  }

  @override
  Future<MenuHandle> registerMenu(
    Menu menu,
    // ignore: avoid_renaming_method_parameters, no_leading_underscores_for_local_identifiers
    MenuSerializationOptions _options,
  ) async {
    final options = MenuSerializationOptions(
      _options.iconTheme.copyWith(size: _platformIconSize()),
      _options.devicePixelRatio,
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

  void _updatePreviewImage(int configurationId, ui.Image previewImage) async {
    final imageData = await ImageData.fromImage(previewImage);
    _channel.invokeMethod('updatePreviewImage', {
      'configurationId': configurationId,
      'image': imageData.serialize(),
    });
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'getConfigurationForLocation') {
      final arguments = call.arguments as Map;
      final offset = OffsetExt.deserialize(arguments['location']);
      final configurationId = arguments['configurationId'] as int;
      final configuration = await delegate?.getMenuConfiguration(
        MenuConfigurationRequest(
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
    } else if (call.method == 'onAction') {
      final actionId = call.arguments as int;
      final element = _elementWithId(actionId)?.element;
      if (element is MenuAction) {
        element.callback();
      }
    } else if (call.method == 'onShowMenu') {
      delegate?.onShowMenu(call.arguments as int);
    } else if (call.method == 'onHideMenu') {
      delegate?.onHideMenu(call.arguments as int);
    } else if (call.method == 'onPreviewAction') {
      delegate?.onPreviewAction(call.arguments as int);
    } else if (call.method == 'getDeferredMenu') {
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

extension MenuConfigurationExt on MenuConfiguration {
  Future<dynamic> serialize() async => {
        'configurationId': configurationId,
        'previewImage': previewImage != null
            ? (await ImageData.fromImage(previewImage!)).serialize()
            : null,
        'previewSize': previewSize?.serialize(),
        'liftImage': (await liftImage.intoRaw()).serialize(),
        'menuHandle': (handle as NativeMenuHandle).handle,
      };
}
