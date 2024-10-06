## 0.8.23

 - **FIX**: workaround for deadlock on iOS 18 (#449).

## 0.8.23-dev.1

 - **FEAT**(macOS): preliminary support for writing tools (#441).

## 0.8.22

 - **FIX**: don't panic with thread local AccessError when shutting down (#426).

## 0.8.21

 - **FIX**: Avoid adding duplicate listeners for drag-n-drop on the web (#422).
 - **FIX**: compilation error on web with latest Flutter main (#425).

## 0.8.20

 - **FIX**: do not build release binary with nightly (#412).
 - **FIX**: do not build release binary with nightly (#412).

## 0.8.19

 - **FIX**: do not build release binary with nightly (#412).
 - **FIX**: panic in ANSI branch of extract_drop_files (#404).
 - **FIX**: synthesize_button_up might cause crash on Linux (#394).

## 0.8.18

 - **FIX**: dragging stuck on web when cancelled too quickly (#398).
 - **FIX**: paste caused crash when clipboard is empty on linux  (#393).

## 0.8.17

 - **FIX**: hide menu drag preview immediately when pan gesture detected (#385).
 - **FIX**: invalid javascript object cast (#380).
 - **FIX**: context menu on iPad with universal control (#378).

## 0.8.16

 - **FIX**: detect drag cancelled on desktop while waiting for data (#377).
 - **FIX**: use startDragAndDrop instead of startDrag on Android sdk24 and above (#372).

## 0.8.15

 - **FIX**: remove obsolete code (#364).

## 0.8.14

 - Bump "super_native_extensions" to `0.8.14`.

## 0.8.13

## 0.8.12

 - Bump "super_native_extensions" to `0.8.12`.

## 0.8.11

 - **FIX**: ignore scroll event in web drag driver.
 - **FIX**: ignore unknown pointer device kind (#344).

## 0.8.10

## 0.8.9

 - **FIX**: delay menu fade-out on iOS (#333).

## 0.8.8

 - **FIX**: regression with custom snapshot (#330).

## 0.8.7

## 0.8.6

 - **FIX**: various exceptions when getting snapshots (#327).
 - **FIX**: fit menu position to bounds after inflating (#322).
 - **FIX**: assertion when taking snapshot of material widget (#320).

## 0.8.5

## 0.8.4

## 0.8.3

## 0.8.2+1

 - **FIX**: remove leftover logging (#284).

## 0.8.2

 - **FIX**: [android] possible deadlock when reading from clipboard (#282).
 - **FEAT**: improve performance with large number of items (#283).
 - **FEAT**: improve performance with large number of items (#274).

## 0.8.1

 - **FIX**: [ios] store user interaction properly (#272).
 - **FIX**: no security scope NSURL access on macos (#271).
 - **FEAT**: [windows] cache file descriptors in reader (#266).

## 0.8.0

 - **FIX**: access NSURL within security scope (#264).

## 0.8.0-dev.3

 - **FEAT**: implement copy and cut events (#253).

## 0.8.0-dev.2

 - Bump "super_native_extensions" to `0.8.0-dev.2`.

## 0.8.0-dev.1

 - **FEAT**: preventDefault for paste event (#249).
 - **FEAT**: implement paste event on web (#246).
 - **FEAT**: migrate to objc2 (#239).

## 0.7.3

 - **FIX**: let zone handle menu callback errors (#228).
 - **FEAT**: improve touch device detection (#227).

## 0.7.2

 - Bump "super_native_extensions" to `0.7.2`.

## 0.7.1

## 0.7.0

## 0.7.0-dev.7

## 0.7.0-dev.6

 - **FIX**: remove trailing null terminator from NSString (#207).
 - **FIX**: [iOS] crash when deferred image is set too quickly (#206).

## 0.7.0-dev.5

 - Bump "super_native_extensions" to `0.7.0-dev.5`.

## 0.7.0-dev.4

## 0.7.0-dev.3

 - **FIX**: [macOS] missing image on NSMenuItem with children (#197).

## 0.7.0-dev.2

 - **FIX**: multi-touch issues on Android (#196).
 - **FIX**: improve closing of menu overlay on touch devices (#193).

## 0.7.0-dev.1

## 0.6.4

 - **FIX**: update engine_context dependency.
 - **FIX**: escape script invocation in podspec.

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
