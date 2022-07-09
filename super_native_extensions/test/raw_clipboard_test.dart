import 'package:nativeshell_core/nativeshell_core.dart';
import 'package:super_native_extensions/src/context.dart';
import 'package:super_native_extensions/raw_clipboard.dart';
import 'package:test/test.dart';

void main() {
  final context = MockMessageChannelContext();
  setUp(() {
    setContextOverride(context);
  });
  test('testReader', () async {
    const channel = 'ClipboardReaderManager';
    var newDefaultReaderCalled = false;
    var disposeReaderCalled = false;
    var getItemsCalled = false;
    var getItemTypesCalled = false;
    var getDataCalled = false;
    context.registerMockMethodCallHandler(channel, (call) {
      if (call.method == 'newDefaultReader') {
        newDefaultReaderCalled = true;
        return {
          'handle': 10,
          'finalizableHandle': FinalizableHandle(-1),
        };
      }
      if (call.method == 'disposeReader') {
        expect(call.arguments, equals(10));
        disposeReaderCalled = true;
        return null;
      }
      if (call.method == 'getItems') {
        expect(call.arguments, equals(10));
        getItemsCalled = true;
        return [1, 2, 3];
      }
      if (call.method == 'getItemTypes') {
        expect(
            call.arguments,
            equals({
              'itemHandle': 2,
              'readerHandle': 10,
            }));
        getItemTypesCalled = true;
        return ['type1', 'type2'];
      }
      if (call.method == 'getItemData') {
        expect(
            call.arguments,
            equals({
              'itemHandle': 2,
              'readerHandle': 10,
              'dataType': 'type1',
            }));
        getDataCalled = true;
        return 'data';
      }
      assert(false, 'Unexpected call $call');
    });
    final reader = await RawClipboardReader.newDefaultReader();
    expect(newDefaultReaderCalled, isTrue);
    final items = await reader.getItems();
    expect(getItemsCalled, isTrue);
    expect(items.length, equals(3));
    final types = await items[1].getAvailableTypes();
    expect(getItemTypesCalled, isTrue);
    expect(types, equals(['type1', 'type2']));
    final data = await items[1].getDataForType('type1');
    expect(getDataCalled, isTrue);
    expect(data, equals('data'));
    await reader.dispose();
    expect(disposeReaderCalled, isTrue);
  });
  // test('TestWriter', () async {
  //   var registerCalled = false;
  //   var writeCalled = false;
  //   var receivedLazyData = false;
  //   var unregisterCalled = false;
  //   const channel = 'ClipboardWriterManager';
  //   context.registerMockMethodCallHandler(channel, (call) async {
  //     if (call.method == 'registerClipboardWriter') {
  //       final expected = {
  //         'items': [
  //           {
  //             'data': [
  //               {
  //                 'type': 'simple',
  //                 'types': ['t1', 't2'],
  //                 'data': 'Data'
  //               },
  //               {
  //                 'type': 'lazy',
  //                 'id': 1,
  //                 'types': ['t1']
  //               },
  //             ]
  //           }
  //         ]
  //       };
  //       expect(call.arguments, equals(expected));
  //       registerCalled = true;
  //       return 10;
  //     }
  //     if (call.method == 'writeToClipboard') {
  //       expect(call.arguments, equals(10));
  //       writeCalled = true;
  //       final lazy = await context.invokeMethod(channel, 'getLazyData', 1);
  //       expect(lazy, equals({'type': 'ok', 'value': 'LazyValue'}));
  //       receivedLazyData = true;
  //       return null;
  //     }
  //     if (call.method == 'unregisterClipboardWriter') {
  //       expect(call.arguments, equals(10));
  //       unregisterCalled = true;
  //       return null;
  //     }
  //     assert(false, 'Unexpected call $call');
  //   });

  //   final data = RawDataSource([
  //     RawClipboardWriterItem([
  //       RawClipboardWriterItemData.simple(types: ['t1', 't2'], data: 'Data'),
  //       RawClipboardWriterItemData.lazy(
  //           types: ['t1'],
  //           dataProvider: () {
  //             return 'LazyValue';
  //           })
  //     ]),
  //   ]);
  //   final writer = await RawClipboardWriter.withData(data);
  //   expect(registerCalled, isTrue);
  //   await writer.writeToClipboard();
  //   expect(writeCalled, isTrue);
  //   expect(receivedLazyData, isTrue);
  //   await writer.dispose();
  //   expect(unregisterCalled, isTrue);
  // });
}
