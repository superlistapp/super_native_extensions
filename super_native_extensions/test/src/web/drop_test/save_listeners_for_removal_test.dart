@TestOn('chrome')

import 'dart:js_interop';
import 'dart:js_interop_unsafe';

import 'package:flutter_test/flutter_test.dart';
import 'package:super_native_extensions/src/web/drop.dart';
import 'package:web/web.dart' as web;

void main() {
  test('should save listeners for removal in the next initialization',
      () async {
    final context = DropContextImpl();
    await context.initialize();
    expect(
      web.document.getProperty(DropContextImpl.listenersProperty.toJS),
      hasLength(4),
    );
  });
}
