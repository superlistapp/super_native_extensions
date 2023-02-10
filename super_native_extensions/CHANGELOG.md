## 0.2.1

 - **FIX**: regression when dropping plain text on web (#66).
 - **FIX**: [macos] error instead of panic when no mouse event is found (#60).
 - **FIX**: do not recreate drag and drop contexts on hot reload (#61).
 - **FIX**: lift image being ignored on iOS (#59).
 - **FEAT**: try revoking drop target first on windows (#63).

## 0.2.0

> Note: This release has breaking changes.

 - **FIX**: increase buffer size.
 - **FIX**: serialize invocation of drop events (#49).
 - **FEAT**: declare more well-known formats (#58).
 - **FEAT**: add support for inplace file reading on ios (#55).
 - **FEAT**: cleanup receiving of files (#54).
 - **FEAT**: initialize ole on windows (#51).
 - **BREAKING** **FEAT**: implement unified content receiving (#47).
 - **BREAKING** **FEAT**: refactor format (#46).

## 0.1.8+2

 - **FIX**: window dragging on macos with full size content view (#43).

## 0.1.8+1

 - **FIX**: rare crash while dragging on iOS (#40).

## 0.1.8

 - **FEAT**: prevent finalizer invoked too early in release mode (#38).

## 0.1.7+3

 - **FIX**: make clippy happy (#36).

## 0.1.7+2

 - **FIX**: minor clean-ups.

## 0.1.7+1

 - **FIX**: create phony file in BUILD_PRODUCTS_DIR.

## 0.1.7

 - **FEAT**: migrate to irondash (#27).

## 0.1.6+2

 - **FIX**: FFI errors in flutter tester.
 - **FIX**: Broken buid on iOS with Rust 1.65.
 - **FIX**: Workaround for Xcode warning.
 - **FIX**: Broken buid on iOS with Rust 1.65.
 - **FIX**: Workaround for Xcode warning.
 - **FIX**: Synthetize mouse up event during drag on linux.

## 0.1.6+1

 - **FIX**: Broken buid on iOS with Rust 1.65.
 - **FIX**: Workaround for Xcode warning.

## 0.1.6

- Fix drop hanging on Windows

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

## 0.1.1

* Improve documentation.

## 0.1.0

* Initial public release.
