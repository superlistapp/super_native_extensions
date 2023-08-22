## 0.6.3

## 0.6.2

## 0.6.0

> Note: This release has breaking changes.

 - **FIX**: declare proper output in podspec script phase.
 - **FIX**: update rust dependencies (#170).
 - **FIX**: [windows] handle null terminator in CF_UNICODETEXT (#169).
 - **FIX**: use destructive icon theme when serializing menu images (#162).
 - **FIX**: [windows] keep IDataObjectAsyncCapability reference during drop (#161).
 - **FIX**: [windows] properly handle data objects that don't support async capability (#160).
 - **FIX**: formatting.
 - **FEAT**: improve compatibility with current Flutter main (#163).
 - **BREAKING** **FIX**: correct typos and spelling in code (#156).
 - **BREAKING** **CHORE**: remove Pair and replace it with dart 3 record (#157).

## 0.5.0

> Note: This release has breaking changes.

 - **FIX**: [macos] assertion when loading deferred menu (#152).
 - **FIX**: [macos] control key stuck after context menu closed (#151).
 - **FIX**: web drag avatar shown in non-root overlay (#139).
 - **FIX**: pasting text with semicolon on macOS (#133).
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

## 0.4.0

 - Bump "super_native_extensions" to `0.4.0`.

## 0.3.0+2

 - **FIX**: [win] rewind OLE streams before reading (#117).

## 0.3.0+1

 - **FIX**: [android] local data only dragging not working (#115).

## 0.3.0

 - **FIX**: [android] build failing with proguard enabled (#114).
 - **FIX**: custom snapshot should propagate exception from renderbox (#104).
 - **FIX**: [ios] revert memory leak fix removal (#103).
 - **FIX**: [web] dropping over platform views not working (#99).
 - **FIX**: [ios] use shadow path from correct image (#97).
 - **FIX**: [ios] force separate drag image to account for shadow difference (#92).
 - **FIX**: [web] dragging ocasionally getting stuck (#89).
 - **FIX**: [windows] pasting files from explorer (#88).
 - **FIX**: use unpremultiplied alpha for encoding image data (#85).
 - **FEAT**: allow merging of snapshot prepare requests (#110).
 - **FEAT**: snapshot optimization (#102).
 - **FEAT**: improve snapshot API (#101).
 - **FEAT**: use widget to customize snapshot setting (#100).
 - **FEAT**: [ios] use real shadow path instead of layer shadow (#95).
 - **FEAT**: [ios] remove drag item provider memory leak workaround (#93).
 - **FEAT**: implement drag shadow on all platforms (#87).

## 0.2.4

 - **FEAT**: [macos] receiving virtual files from outlook attachments (#81).

## 0.2.3

 - **FEAT**: add super_hot_key (#77).

## 0.2.2+2

 - **FIX**: [android] throw exception if wrong mime filter is requested.

## 0.2.2+1

 - **FIX**: clipboard copy on web in release mode (#72).
 - **FIX**: [windows] use cached length when reading virtual stream (#69).

## 0.2.2

 - **FIX**: regression when dropping plain text on web (#66).
 - **FIX**: [macos] error instead of panic when no mouse event is found (#60).
 - **FIX**: do not recreate drag and drop contexts on hot reload (#61).
 - **FIX**: lift image being ignored on iOS (#59).
 - **FEAT**: [windows] use thread pool for virtual file background thread (#68).
 - **FEAT**: [windows] delay virtual file request until IStream is read (#67).
 - **FEAT**: try revoking drop target first on windows (#63).

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
 - **FIX**: Synthesize mouse up event during drag on linux.

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
