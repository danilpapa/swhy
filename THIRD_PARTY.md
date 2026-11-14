# Third-party material

This file lists material that is copied or adapted from other projects into the
`swhy` source tree, with its license notice. Ordinary Cargo and npm
dependencies are not listed here: their licenses travel with the packages.

## Word lists from the Swift compiler

**Status:** nothing from the Swift compiler is in the repository yet. The lists
are brought in at stage E9 (Objective-C to Swift rewrites); this section is
completed in the same commit that adds them.

| | |
| --- | --- |
| Project | The Swift Programming Language, <https://github.com/swiftlang/swift> |
| What is borrowed | Lists of prepositions and verbs from `lib/Basic/PartsOfSpeech.def`; the name-splitting logic of `lib/Basic/StringExtras.cpp` (`omitNeedlessWords`, `splitBaseNameAfterLastPreposition`) and `lib/ClangImporter/ImportName.cpp` is re-implemented in Rust |
| Where it goes | `crates/swhy-core/src/track/crosslang.rs` and its data files |
| License | Apache License 2.0 with Runtime Library Exception |
| Upstream revision | to be recorded at E9 |

Notice to keep with the borrowed lists:

```
This source file is part of the Swift.org open source project

Copyright (c) 2014 - 2017 Apple Inc. and the Swift project authors
Licensed under Apache License v2.0 with Runtime Library Exception

See https://swift.org/LICENSE.txt for license information
See https://swift.org/CONTRIBUTORS.txt for the list of Swift project authors
```

The text of the Apache License 2.0 is in `LICENSE-APACHE`. The Runtime Library
Exception reads:

```
As an exception, if you use this Software to compile your source code and
portions of this Software are embedded into the binary product as a result,
you may redistribute such product without providing attribution as would
otherwise be required by Sections 4(a), 4(b) and 4(d) of the License.
```
