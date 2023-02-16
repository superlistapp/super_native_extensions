## 0.2.2

 - **FEAT**: [windows] improved behavior when dragging virtual files over certain applications

## 0.2.1

 - **FIX**: regression when dropping plain text on web (#66).
 - **FIX**: [macos] error instead of panic when no mouse event is found (#60).
 - **FIX**: do not recreate drag and drop contexts on hot reload (#61).
 - **FIX**: lift image being ignored on iOS (#59).
 - **FEAT**: try revoking drop target first on windows (#63).

## 0.2.0

> Note: This release has breaking changes.

 - **FIX**: properly unitialize com on windows.
 - **FEAT**: cleanup receiving of files (#54).
 - **FEAT**: initialize ole on windows (#51).
 - **DOCS**: minor fix.
 - **BREAKING** **FEAT**: implement unified content receiving (#47).
 - **BREAKING** **FEAT**: refactor format (#46).

## 0.1.9+3

 - Update a dependency to the latest release.

## 0.1.9+2

 - Update a dependency to the latest release.

## 0.1.9+1

 - Update a dependency to the latest release.

## 0.1.9

 - **FEAT**: [drop] add support for slivers (#35).

## 0.1.8+1

 - **FIX**: minor clean-ups.

## 0.1.8

 - **FIX**: cancel mouse hover during dragging (#34).
 - **FEAT**: super_drag_and_drop: reexport formats from super_clipboard (#32).
 - **FEAT**: expose isLocationDraggable  for DraggableWidget (#31).

## 0.1.7+1

 - Update a dependency to the latest release.

## 0.1.7

 - **FEAT**: migrate to irondash (#27).

## 0.1.6+2

 - **FIX**: Workaround for Xcode warning.

## 0.1.6+1

 - **FIX**: Workaround for Xcode warning.

## 0.1.5

- Fix compatibility with NDK 23+

## 0.1.4

 - **FEAT**: add_super_keyboard_layout (#20).

## 0.1.3+1

 - **FIX**: Improve Drag&Drop on Web (#19).

## 0.1.3

* Improve documentation.

## 0.1.2

* Improve documentation.
* Use event object in DropRegion and DropMonitor callbacks.

## 0.1.1

* Improve documentation.

## 0.1.0

* Initial public release.
