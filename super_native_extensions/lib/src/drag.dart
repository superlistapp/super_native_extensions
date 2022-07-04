import 'dart:convert';
import 'dart:io';
import 'dart:ui' as ui;

import 'package:flutter/services.dart';
import 'package:flutter/widgets.dart';
import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/raw_clipboard.dart';
import 'package:super_native_extensions/src/context.dart';

import 'mutex.dart';
import 'api_model.dart';

abstract class RawDragContextDelegate {
  Future<DataSourceHandle?> getDataSourceForDragRequest({ui.Offset location});
}

final _channel =
    NativeMethodChannel('DragManager', context: superNativeExtensionsContext);

class RawDragContext {
  RawDragContext._();

  static RawDragContext? _instance;
  static final _mutex = Mutex();

  Future<void> _initialize() async {
    WidgetsFlutterBinding.ensureInitialized();
    final view = await getFlutterView();
    _channel.setMethodCallHandler(_handleMethodCall);
    await _channel.invokeMethod("newContext", {'viewHandle': view});
  }

  static Future<RawDragContext> instance() {
    return _mutex.protect(() async {
      if (_instance == null) {
        _instance = RawDragContext._();
        await _instance!._initialize();
      }
      return _instance!;
    });
  }

  Future<dynamic> _handleMethodCall(MethodCall call) async {
    if (call.method == 'dataSourceForDragRequest') {
      print('ARG ${call.arguments}');
      final data = DataSource([
        DataSourceItem(representations: [
          // DataSourceItemRepresentation.lazy(
          //   formats: ['public.url'],
          //   // data: utf8.encode('https://airflow.app'),
          //   dataProvider: (_) => utf8.encode('https://airflow.app'),
          // )
          DataSourceItemRepresentation.virtualFile(
              format: 'public.utf8-plain-text',
              storageSuggestion: VirtualFileStorage.temporaryFile,
              virtualFileProvider: (sink, progress) {
                final cancelled = [false];
                print('Requested file');
                progress.onCancel.addListener(() {
                  print('Cancelled');
                  cancelled[0] = true;
                });
                for (var i = 0; i < 10; ++i) {
                  Future.delayed(Duration(milliseconds: i * 1000), () {
                    if (cancelled[0]) {
                      return;
                    }
                    progress.updateProgress(i * 10);
                    if (i == 9) {
                      print('Done');
                      sink.add(utf8.encode('Hello, cruel world!\n'));
                      sink.add(utf8.encode('Hello, cruel world!'));
                      // sink.addError('Something went wrong');
                      sink.close();
                    }
                  });
                }
              }),
        ], suggestedName: 'File1.txt'),
        // DataSourceItem(representations: [
        //   DataSourceItemRepresentation.virtualFile(
        //       format: 'public.utf8-plain-text',
        //       virtualFileProvider: (targetPath, progress, onComplete, onError) {
        //         final cancelled = [false];
        //         print('Requested file at path 2 $targetPath');
        //         progress.onCancel.addListener(() {
        //           print('Cancelled 2');
        //           cancelled[0] = true;
        //         });
        //         for (var i = 0; i < 10; ++i) {
        //           Future.delayed(Duration(milliseconds: i * 1000), () {
        //             if (cancelled[0]) {
        //               return;
        //             }
        //             progress.updateProgress(i * 10);
        //             if (i == 9) {
        //               print('Done 2');
        //               final file = File(targetPath);
        //               file.writeAsStringSync('Hello world 22');
        //               onComplete();
        //             }
        //           });
        //         }
        //       }),
        // ], suggestedName: 'File2.txt'),
      ]);
      // final writer = await RawClipboardWriter.withData(data);
      final handle = await data.register();
      return {'dataSourceId': handle.id};
    } else if (call.method == 'releaseDataSource') {
      print('Release source ${call.arguments as int}');
    } else {
      return null;
    }
  }

  Future<void> startDrag({
    required DragRequest request,
  }) async {
    return _channel.invokeMethod("startDrag", await request.serialize());
  }
}

class DragRequest {
  DragRequest({
    required this.dataSource,
    required this.pointInRect,
    required this.image,
  });

  final DataSourceHandle dataSource;
  final Offset pointInRect;
  final ui.Image image;

  Future<dynamic> serialize() async {
    final imageData = await ImageData.fromImage(image);
    return {
      'dataSource': dataSource.id,
      'pointInRect': pointInRect.serialize(),
      'image': imageData.serialize(),
    };
  }
}
