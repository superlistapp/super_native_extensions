import 'dart:ui' as ui;

import 'package:flutter/material.dart';
import 'package:super_context_menu/super_context_menu.dart';
import 'package:super_drag_and_drop/super_drag_and_drop.dart';
import 'package:super_native_extensions/src/api_model.dart';

late MenuContext menuContext;

Future<ImageData> _createImageData(Color color) async {
  final recorder = ui.PictureRecorder();
  final canvas = Canvas(recorder);
  final paint = Paint()..color = color;
  canvas.drawOval(const Rect.fromLTWH(0, 0, 200, 200), paint);
  final picture = recorder.endRecording();
  final image = await picture.toImage(200, 200);
  return ImageData.fromImage(image, devicePixelRatio: 2.0);
}

MenuHandle? _handle;

class Delegate extends MenuContextDelegate {
  @override
  Future<MenuConfiguration?> getConfigurationForLocation({
    required Offset location,
  }) async {
    final configuration = MenuConfiguration(
        image: TargettedImageData(
          imageData: await _createImageData(Colors.red),
          rect: const Rect.fromLTWH(100, 100, 100, 100),
        ),
        liftImage: TargettedImageData(
          imageData: await _createImageData(Colors.blue),
          rect: const Rect.fromLTWH(100, 100, 100, 100),
        ),
        handle: _handle!);
    print("Get configuration for $location");
    return configuration;
  }
}

void main() async {
  menuContext = await MenuContext.instance();
  menuContext.delegate = Delegate();
  _handle = await menuContext.registerMenu(Menu(
    children: [
      MenuAction(
        title: 'Action 1',
        attributes: MenuElementAttributes(destructive: true),
        image: await _createImageData(Colors.green),
        callback: () {
          print('Action 1');
        },
      ),
      DeferredMenuElement(() async {
        await Future.delayed(const Duration(seconds: 2));
        return [
          Separator(title: 'Deferred Submenu'),
          MenuAction(
            title: 'Action D',
            callback: () {
              print('ActionD');
            },
          ),
          Menu(title: 'Inside', children: [
            MenuAction(
              title: 'Action X',
              callback: () {
                print('ActionE');
              },
            ),
          ]),
          Separator(),
        ];
      }),
      Menu(
        title: 'Submenu',
        children: [
          for (var i = 0; i < 30; ++i)
            MenuAction(
              title: 'Action 2',
              callback: () {
                print('Action2');
              },
            ),
          Menu(
            title: 'Submenu',
            children: [
              MenuAction(
                title: 'Action 2',
                callback: () {
                  print('Action2');
                },
              ),
              Separator(title: 'Submenu'),
              MenuAction(
                title: 'Action 3',
                callback: () {
                  print('Action 3');
                },
              ),
            ],
          ),
          Separator(title: 'Submenu'),
          MenuAction(
            title: 'Action 3',
            callback: () {
              print('Action 3');
            },
          ),
        ],
      ),
    ],
  ));
  runApp(const MainApp());
}

class MainApp extends StatelessWidget {
  const MainApp({super.key});

  @override
  Widget build(BuildContext context) {
    return MaterialApp(
      home: Scaffold(
        body: Stack(
          children: [
            Positioned.fromRect(
              rect: const Rect.fromLTWH(100, 100, 100, 100),
              child: DragItemWidget(
                dragItemProvider: (request) async {
                  return DragItem(
                      image: request.dragImage(), localData: 'Hello World!');
                },
                allowedOperations: () {
                  return [DropOperation.copy];
                },
                child: Listener(
                  onPointerDown: (event) {
                    print('Pointer down');
                  },
                  onPointerUp: (event) {
                    print('Pointer up');
                  },
                  onPointerCancel: (event) {
                    print('pointer cancel');
                  },
                  child: DraggableWidget(
                    hitTestBehavior: HitTestBehavior.translucent,
                    child: Container(
                      color: Colors.amber,
                      padding: const EdgeInsets.all(20),
                      child: const Text('Hello World!'),
                    ),
                  ),
                ),
              ),
            ),
          ],
        ),
      ),
    );
  }
}
