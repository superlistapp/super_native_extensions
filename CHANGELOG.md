# Change Log

All notable changes to this project will be documented in this file.
See [Conventional Commits](https://conventionalcommits.org) for commit guidelines.

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

