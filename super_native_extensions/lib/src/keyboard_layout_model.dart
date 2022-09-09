class KeyboardKey {
  KeyboardKey({
    required this.platform,
    required this.physical,
    this.logical,
    this.logicalShift,
    this.logicalAlt,
    this.logicalAltShift,
    this.logicalMeta,
  });

  final int platform;
  final int physical;
  final int? logical;
  final int? logicalShift;
  final int? logicalAlt;
  final int? logicalAltShift;
  final int? logicalMeta;

  static KeyboardKey deserialize(dynamic value) {
    final map = value as Map;
    return KeyboardKey(
        platform: map['platform'],
        physical: map['physical'],
        logical: map['logical'],
        logicalShift: map['logicalShift'],
        logicalAlt: map['logicalAlt'],
        logicalAltShift: map['logicalAltShift'],
        logicalMeta: map['logicalMeta']);
  }
}

class KeyboardLayout {
  KeyboardLayout({
    required this.keys,
  });

  final List<KeyboardKey> keys;

  static KeyboardLayout? deserialize(dynamic value) {
    if (value == null) {
      return null;
    }
    final map = value as Map;
    final keys = map['keys'] as List;
    return KeyboardLayout(keys: keys.map(KeyboardKey.deserialize).toList());
  }
}
