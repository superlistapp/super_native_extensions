## Features

System-wide hotkeys for macOS and Windows.

## Usage

```dart
final hotKey = await HotKey.create(
  definition: HotKeyDefinition(
    key: PhysicalKeyboardKey.minus,
    alt: true,
    meta: true,
  ),
  callback: () {
    print('hot key pressed');
  },
);

// .. Meta + Alt + Minus will trigger the callback regardless of whether
// the application is in focus

// Unregister the hot key
hotKey.dispose();
```

## Additional information

Hot keys are registered on physical keys. To convert between physical and logical keys you can use the [super_keyboard_layout](https://pub.dev/packages/super_keyboard_layout) package.
