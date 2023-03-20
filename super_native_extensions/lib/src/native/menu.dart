import 'dart:async';
import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:irondash_engine_context/irondash_engine_context.dart';
import 'package:irondash_message_channel/irondash_message_channel.dart';
import 'package:super_native_extensions/src/api_model.dart';
import 'package:super_native_extensions/src/util.dart';

import '../menu.dart';
import 'api_model.dart';
import 'context.dart';

class SerializeOptions {
  SerializeOptions({
    required this.devicePixelRatio,
  });

  final double devicePixelRatio;
}

Future<dynamic> _serializeImage(
    FutureOr<ui.Image?>? image, SerializeOptions options) async {
  if (image is Future<ui.Image?>) {
    image = await image;
  }
  if (image is ui.Image) {
    return (await ImageData.fromImage(image,
            devicePixelRatio: options.devicePixelRatio))
        .serialize();
  } else {
    return null;
  }
}

extension on Menu {
  Future<dynamic> serialize(SerializeOptions options) async => {
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

extension on MenuElementAttributes {
  Future<dynamic> serialize(SerializeOptions option) async => {
        'disabled': disabled,
        'destructive': destructive,
      };
}

extension on Separator {
  Future<dynamic> serialize(SerializeOptions options) async => {
        'type': 'separator',
        'content': {
          'title': title,
        }
      };
}

extension on MenuAction {
  Future<dynamic> serialize(SerializeOptions options) async => {
        'type': 'action',
        'content': {
          'uniqueId': uniqueId,
          'identifier': identifier,
          'title': title,
          'subtitle': subtitle,
          'image': await _serializeImage(image, options),
          'attributes': await attributes.serialize(options),
        }
      };
}

extension on DeferredMenuElement {
  Future<dynamic> serialize(SerializeOptions options) async => {
        'type': 'deferred',
        'content': {
          'uniqueId': uniqueId,
        }
      };
}

extension on MenuElement {
  Future<dynamic> serialize(SerializeOptions options) {
    if (this is Menu) {
      return (this as Menu).serialize(options);
    } else if (this is MenuAction) {
      return (this as MenuAction).serialize(options);
    } else if (this is DeferredMenuElement) {
      return (this as DeferredMenuElement).serialize(options);
    } else if (this is Separator) {
      return (this as Separator).serialize(options);
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
    required this.elements,
    required this.onDispose,
    required this.devicePixelRatio,
  });

  final int handle;
  final List<MenuElement> elements;
  final VoidCallback onDispose;
  final double devicePixelRatio;

  @override
  void dispose() {
    onDispose();
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

  @override
  Future<MenuHandle> registerMenu(Menu menu) async {
    final options = SerializeOptions(devicePixelRatio: 2.0);
    // The cast is necessary for correct extension method to be called.
    // ignore: unnecessary_cast
    final serialized = await (menu as MenuElement).serialize(options);
    final handle =
        await _channel.invokeMethod('registerMenu', serialized) as int;
    final res = NativeMenuHandle(
      elements: [menu],
      handle: handle,
      onDispose: () {
        _handles.removeWhere((element) => element.handle == handle);
        _channel.invokeMethod('disposeMenu', handle);
      },
      devicePixelRatio: options.devicePixelRatio,
    );
    _handles.add(res);
    return res;
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
        ),
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
        final menu = await (element.element as DeferredMenuElement).provider();
        res = await Future.wait(menu.map((e) {
          element.handle.elements.add(e);
          return e.serialize(SerializeOptions(
            devicePixelRatio: element.handle.devicePixelRatio,
          ));
        }));
      }
      return {'elements': res};
    } else {
      return null;
    }
  }
}

extension MenuConfigurationExt on MenuConfiguration {
  dynamic serialize() => {
        'configurationId': configurationId,
        'image': image.serialize(),
        'liftImage': liftImage?.serialize(),
        'menuHandle': (handle as NativeMenuHandle).handle,
      };
}
