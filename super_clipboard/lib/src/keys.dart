import 'keys_internal.dart';

const plaintextKey = SimpleClipboardKey<String>(
  ios: SimpleClipboardPlatformKey<String>(
    types: ['public/utf8-plain-text'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  macos: SimpleClipboardPlatformKey<String>(
    types: ['public/utf8-plain-text'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  android: SimpleClipboardPlatformKey<String>(
    types: ['text/plain'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  linux: SimpleClipboardPlatformKey<String>(
    types: ['text/plain'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  windows: SimpleClipboardPlatformKey(
      onConvertFromSystem: fromSystemUtf16NullTerminated,
      onConvertToSystem: passthrough,
      types: [
        '${cfInternalPrefix}13' // CF_UNICODETEXT
      ]),
);

/// Key for rich text in form of html snippet. Make sure to include `plaintextKey`
/// version in clipboard as well, otherwise setting the content may fail on some
/// platforms (i.e. Android).
const htmlFragmentKey = SimpleClipboardKey<String>(
  ios: SimpleClipboardPlatformKey<String>(
    types: ['public/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  macos: SimpleClipboardPlatformKey<String>(
    types: ['public/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  android: SimpleClipboardPlatformKey<String>(
    types: ['text/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  linux: SimpleClipboardPlatformKey<String>(
    types: ['text/html'],
    onConvertFromSystem: fromSystemUtf8,
    onConvertToSystem: passthrough,
  ),
  windows: SimpleClipboardPlatformKey(
      onConvertFromSystem: windowsHtmlFromSystem,
      onConvertToSystem: windowsHtmlToSystem,
      types: [
        'text/html',
        cfHtml,
      ]),
);
