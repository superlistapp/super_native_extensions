# Change Log

All notable changes to this project will be documented in this file.
See [Conventional Commits](https://conventionalcommits.org) for commit guidelines.

## 2023-08-22

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_clipboard` - `v0.6.2`](#super_clipboard---v062)
 - [`super_drag_and_drop` - `v0.6.2`](#super_drag_and_drop---v062)
 - [`super_context_menu` - `v0.6.2`](#super_context_menu---v062)
 - [`super_native_extensions` - `v0.6.2`](#super_native_extensions---v062)
 - [`super_hot_key` - `v0.6.2`](#super_hot_key---v062)
 - [`super_keyboard_layout` - `v0.6.2`](#super_keyboard_layout---v062)

---

#### `super_clipboard` - `v0.6.2`

 - Bump "super_clipboard" to `0.6.2`.

#### `super_drag_and_drop` - `v0.6.2`

 - Bump "super_drag_and_drop" to `0.6.2`.

#### `super_context_menu` - `v0.6.2`

 - Bump "super_context_menu" to `0.6.2`.

#### `super_native_extensions` - `v0.6.2`

#### `super_hot_key` - `v0.6.2`

 - Bump "super_hot_key" to `0.6.2`.

#### `super_keyboard_layout` - `v0.6.2`

 - Bump "super_keyboard_layout" to `0.6.2`.


## 2023-08-21

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_clipboard` - `v0.6.1`](#super_clipboard---v061)
 - [`super_drag_and_drop` - `v0.6.1`](#super_drag_and_drop---v061)
 - [`super_context_menu` - `v0.6.1`](#super_context_menu---v061)

---

#### `super_clipboard` - `v0.6.1`

 - **DOCS**: update comments.

#### `super_drag_and_drop` - `v0.6.1`

 - **DOCS**: update comments.

#### `super_context_menu` - `v0.6.1`

 - **FIX**: [iOS] gesture recognizer workaround (#176).


## 2023-08-07

### Changes

---

Packages with breaking changes:

 - [`super_clipboard` - `v0.6.0`](#super_clipboard---v060)
 - [`super_drag_and_drop` - `v0.6.0`](#super_drag_and_drop---v060)
 - [`super_native_extensions` - `v0.6.0`](#super_native_extensions---v060)
 - [`super_context_menu` - `v0.6.0`](#super_context_menu---v060)

Packages with other changes:

 - [`super_keyboard_layout` - `v0.6.0`](#super_keyboard_layout---v060)
 - [`super_hot_key` - `v0.6.0`](#super_hot_key---v060)

---

#### `super_clipboard` - `v0.6.0`

 - **FIX**: correct imports and add missing exports (#155).
 - **FEAT**: improve compatibility with current Flutter main (#163).
 - **BREAKING** **FIX**: correct typos and spelling in code (#156).
 - **BREAKING** **CHORE**: remove Pair and replace it with dart 3 record (#157).

#### `super_drag_and_drop` - `v0.6.0`

 - **FEAT**: improve compatibility with current Flutter main (#163).
 - **BREAKING** **FIX**: correct typos and spelling in code (#156).

#### `super_native_extensions` - `v0.6.0`

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

#### `super_context_menu` - `v0.6.0`

 - **FIX**: use destructive icon theme when serializing menu images (#162).
 - **FEAT**: improve compatibility with current Flutter main (#163).
 - **BREAKING** **FIX**: correct typos and spelling in code (#156).

#### `super_keyboard_layout` - `v0.6.0`

 - **FEAT**: improve compatibility with current Flutter main (#163).

#### `super_hot_key` - `v0.6.0`

 - Bump "super_hot_key" to `0.6.0`.


## 2023-07-22

### Changes

---

Packages with breaking changes:

 - [`super_clipboard` - `v0.5.0`](#super_clipboard---v050)
 - [`super_context_menu` - `v0.5.0`](#super_context_menu---v050)
 - [`super_drag_and_drop` - `v0.5.0`](#super_drag_and_drop---v050)
 - [`super_hot_key` - `v0.5.0`](#super_hot_key---v050)
 - [`super_keyboard_layout` - `v0.5.0`](#super_keyboard_layout---v050)
 - [`super_native_extensions` - `v0.5.0`](#super_native_extensions---v050)

Packages with other changes:

 - There are no other changes in this release.

---

#### `super_clipboard` - `v0.5.0`

 - **DOCS**: [android] mention minSdkVersion in readme (#150).
 - **DOCS**: update NDK installation information (#149).
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

#### `super_context_menu` - `v0.5.0`

 - **FIX**: context menu in list view not working on iOS (#144).
 - **FEAT**: implement safe triangle for desktop menu (#153).
 - **DOCS**: update NDK installation information (#149).
 - **DOCS**: fixup unnecessary capitalization.
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

#### `super_drag_and_drop` - `v0.5.0`

 - **FIX**: ensure drop regions are attached when invoking events (#147).
 - **FIX**: cache active items for snapshotter (#146).
 - **DOCS**: [android] mention minSdkVersion in readme (#150).
 - **DOCS**: update NDK installation information (#149).
 - **DOCS**: fix example.
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

#### `super_hot_key` - `v0.5.0`

 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

#### `super_keyboard_layout` - `v0.5.0`

 - **DOCS**: update NDK installation information (#149).
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).

#### `super_native_extensions` - `v0.5.0`

 - **FIX**: [macos] assertion when loading deferred menu (#152).
 - **FIX**: [macos] control key stuck after context menu closed (#151).
 - **FIX**: web drag avatar shown in non-root overlay (#139).
 - **FIX**: pasting text with semicolon on macOS (#133).
 - **BREAKING** **FEAT**: upgrade to Dart 3 and jni 0.21.1 (#138).


## 2023-05-22

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_clipboard` - `v0.4.0`](#super_clipboard---v040)
 - [`super_drag_and_drop` - `v0.4.0`](#super_drag_and_drop---v040)
 - [`super_native_extensions` - `v0.4.0`](#super_native_extensions---v040)
 - [`super_keyboard_layout` - `v0.4.0`](#super_keyboard_layout---v040)
 - [`super_hot_key` - `v0.4.0`](#super_hot_key---v040)
 - [`super_context_menu` - `v0.1.0`](#super_context_menu---v010)

---

#### `super_clipboard` - `v0.4.0`

 - Bump "super_clipboard" to `0.4.0`.

#### `super_drag_and_drop` - `v0.4.0`

 - Bump "super_drag_and_drop" to `0.4.0`.

#### `super_native_extensions` - `v0.4.0`

 - Bump "super_native_extensions" to `0.4.0`.

#### `super_keyboard_layout` - `v0.4.0`

 - Bump "super_keyboard_layout" to `0.4.0`.

#### `super_hot_key` - `v0.4.0`

 - Bump "super_hot_key" to `0.4.0`.

#### `super_context_menu` - `v0.1.0`

 - Bump "super_context_menu" to `0.1.0`.


## 2023-04-03

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_native_extensions` - `v0.3.0+2`](#super_native_extensions---v0302)
 - [`super_clipboard` - `v0.3.0+2`](#super_clipboard---v0302)
 - [`super_drag_and_drop` - `v0.3.0+2`](#super_drag_and_drop---v0302)
 - [`super_hot_key` - `v0.3.0+2`](#super_hot_key---v0302)
 - [`super_keyboard_layout` - `v0.3.0+2`](#super_keyboard_layout---v0302)

Packages with dependency updates only:

> Packages listed below depend on other packages in this workspace that have had changes. Their versions have been incremented to bump the minimum dependency versions of the packages they depend upon in this project.

 - `super_clipboard` - `v0.3.0+2`
 - `super_drag_and_drop` - `v0.3.0+2`
 - `super_hot_key` - `v0.3.0+2`
 - `super_keyboard_layout` - `v0.3.0+2`

---

#### `super_native_extensions` - `v0.3.0+2`

 - **FIX**: [win] rewind OLE streams before reading (#117).


## 2023-03-30

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_native_extensions` - `v0.3.0+1`](#super_native_extensions---v0301)
 - [`super_clipboard` - `v0.3.0+1`](#super_clipboard---v0301)
 - [`super_drag_and_drop` - `v0.3.0+1`](#super_drag_and_drop---v0301)
 - [`super_keyboard_layout` - `v0.3.0+1`](#super_keyboard_layout---v0301)
 - [`super_hot_key` - `v0.3.0+1`](#super_hot_key---v0301)

Packages with dependency updates only:

> Packages listed below depend on other packages in this workspace that have had changes. Their versions have been incremented to bump the minimum dependency versions of the packages they depend upon in this project.

 - `super_clipboard` - `v0.3.0+1`
 - `super_drag_and_drop` - `v0.3.0+1`
 - `super_keyboard_layout` - `v0.3.0+1`
 - `super_hot_key` - `v0.3.0+1`

---

#### `super_native_extensions` - `v0.3.0+1`

 - **FIX**: [android] local data only dragging not working (#115).


## 2023-03-29

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_clipboard` - `v0.3.0`](#super_clipboard---v030)
 - [`super_drag_and_drop` - `v0.3.0`](#super_drag_and_drop---v030)
 - [`super_native_extensions` - `v0.3.0`](#super_native_extensions---v030)
 - [`super_keyboard_layout` - `v0.3.0`](#super_keyboard_layout---v030)
 - [`super_hot_key` - `v0.3.0`](#super_hot_key---v030)

---

#### `super_clipboard` - `v0.3.0`

 - **FIX**: [android] build failing with proguard enabled (#114).
 - **FEAT**: add htmlFile format (#107).
 - **FEAT**: make format in DataReader.getFile optional (#90).

#### `super_drag_and_drop` - `v0.3.0`

 - **FIX**: [android] build failing with proguard enabled (#114).
 - **FIX**: [ios] respect isLocationDraggable check (#109).
 - **FIX**: super_drag_and_drop should reexport Format (#83).
 - **FEAT**: allow merging of snapshot prepare requests (#110).
 - **FEAT**: simplify lift snapshot logic on iOS (#108).
 - **FEAT**: improve snapshot API (#101).
 - **FEAT**: use widget to customize snapshot setting (#100).
 - **FEAT**: implement drag shadow on all platforms (#87).
 - **DOCS**: fix typo.
 - **DOCS**: improve super_drag_and_drop documentation (#106).

#### `super_native_extensions` - `v0.3.0`

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

#### `super_keyboard_layout` - `v0.3.0`

 - **FIX**: [android] build failing with proguard enabled (#114).

#### `super_hot_key` - `v0.3.0`

 - n


## 2023-03-14

### Changes

---

Packages with breaking changes:

 - There are no breaking changes in this release.

Packages with other changes:

 - [`super_native_extensions` - `v0.2.4`](#super_native_extensions---v024)
 - [`super_clipboard` - `v0.2.3+1`](#super_clipboard---v0231)
 - [`super_hot_key` - `v0.1.1+1`](#super_hot_key---v0111)
 - [`super_drag_and_drop` - `v0.2.3+1`](#super_drag_and_drop---v0231)
 - [`super_keyboard_layout` - `v0.2.1+1`](#super_keyboard_layout---v0211)

Packages with dependency updates only:

> Packages listed below depend on other packages in this workspace that have had changes. Their versions have been incremented to bump the minimum dependency versions of the packages they depend upon in this project.

 - `super_clipboard` - `v0.2.3+1`
 - `super_hot_key` - `v0.1.1+1`
 - `super_drag_and_drop` - `v0.2.3+1`
 - `super_keyboard_layout` - `v0.2.1+1`

---

#### `super_native_extensions` - `v0.2.4`

 - **FEAT**: [macos] receiving virtual files from outlook attachments (#81).

