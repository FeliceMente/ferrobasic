# Porting existing B4X projects

A B4X project builds unchanged only if every layer it touches is supported, so
compatibility is defined layer by layer. The JVM and Android targets can reach
near-total compatibility, because
[most B4A and B4J libraries are open source under Apache 2.0](https://www.b4x.com/android/forum/threads/114755)
([AnywhereSoftware on GitHub](https://github.com/AnywhereSoftware)) and can be reused as they are.

## Layers

| Layer | What the compiler must do | Unchanged on the original platform | Unchanged on other targets |
| --- | --- | --- | --- |
| Project files (`.b4j`, `.b4a`, `.b4i`, `.b4r`) | Read the header (modules, libraries, files, build configurations) and the main module after the design-text separator; honour `Files` and shared folders | Yes | Yes |
| Conditional compilation and attributes | Define the same symbols as the original product (`B4J`, `B4A`, build configurations); honour `#Region Project Attributes`, `#AdditionalJar`, `#Macro` | Yes | Yes, with that product's symbols |
| B4X code and `.b4xlib` libraries | Compile from source; `.b4xlib` is a zip of B4X modules and files | Yes | Yes |
| Core libraries (Core, jCore, XUI, B4XPages, JSON, SQL, OkHttpUtils2…) | JVM/Android: link the original Apache 2.0 jars through their `.xml` descriptors. Elsewhere: reimplement the same API and ship the same descriptors | Yes | Yes, once reimplemented |
| Third-party Java libraries (`.jar` + `.xml`) | Link them on JVM/Android, if their licence allows it | Yes | No: clear error |
| Inline code (`#If JAVA`, `#If OBJC`, `#If C`), `JavaObject`, `NativeObject` | Pass through to the matching backend | Yes | No: clear error |
| Layouts (`.bjl`, `.bal`, `.bil`) and designer scripts | Read the binary format, run the designer scripts at runtime | Yes (M6) | Yes (M6), via XUI |

## Products and targets

| Original product | Default target | Notes |
| --- | --- | --- |
| B4J | JVM | Native desktop and WASM when every library used has a native implementation |
| B4A | Android, via the JVM backend | Original Apache 2.0 libraries reused |
| B4i | iOS, via LLVM | B4i internal libraries are not open source, so the iOS API is reimplemented |
| B4R | Microcontrollers, via LLVM or C | Separate profile matching B4R semantics (no GC, static allocation) and the rCore API |

## Rules

- Library `.xml` descriptors are the type-checking interface: the compiler
  reads them to know classes, methods, properties and events. Reimplemented
  libraries ship identical descriptors, so projects type-check the same way on
  every target.
- A project that cannot build unchanged gets a precise diagnostic: which library
  member, inline block or layout feature is missing on that target. It never
  builds into something that behaves differently.
- Cross-platform projects written with B4XPages and XUI are the realistic sweet
  spot for other targets; platform-specific code (Activity, JavaFX Form) stays
  on its original platform.
- Some third-party B4X libraries are freeware with licences that forbid reverse
  engineering and use in other development tools (for example
  [this licence](https://github.com/Mashiane/BANanoVuetifyAD3/blob/main/BANano%20LICENSE.TXT)).
  FerroBASIC never redistributes libraries; users bring their own, and the
  documentation warns about such licences.
- The project and layout formats are undocumented and must be reverse-engineered
  from real projects for interoperability. Check the legal limits with a lawyer
  before publishing the readers.

## Open points

Tracked in [open-questions.md](open-questions.md#porting).
