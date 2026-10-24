# Project and module files

What FerroBASIC knows about the files a B4X-compatible project is made of,
and the sources of that knowledge. Only public files and posts are used; the
IDE and compiler are not examined.

So far this covers what Level 0 needs: the header shared by project files
(`.b4j`) and module files (`.bas`), and the keys that decide whether Level 0
can run a project. The rest of the format (build configurations, shared
modules, `.b4xlib`, other products) is documented as M0 and M3 need it.

## The header

A project file and a module file both start with a header, then the code:

- an optional UTF-8 byte order mark;
- lines `Key=Value`, where a key is ASCII letters, digits and `_`;
- a line `@EndOfDesignText@`;
- then the code: the main module for a project file, the module for a
  module file.

The IDE writes CRLF line ends and, in recent versions, the keys in
alphabetical order. Older files have the keys in another order. The header
is part of the lossless syntax tree (`HEADER` token), so tools keep it byte
for byte.

## Project file (`.b4j`)

A console project, as written by B4J 7 (a public sample; see Sources):

```text
AppType=StandardJava
Build1=Default,b4j.example
Group=Default Group
Library1=jcore
NumberOfFiles=0
NumberOfLibraries=1
NumberOfModules=0
Version=7
@EndOfDesignText@
```

A UI project with a module and files (B4J 8.9, a public sample):

```text
AppType=JavaFX
Build1=Default,b4j.example
File1=1.bjl
File2=logo.png
FileGroup1=New Group
FileGroup2=New Group
Group=Default Group
Library1=b4xpages
Library2=bctextengine
Library3=jcore
Library4=jfx
Module1=|relative|..\B4XMainPage
NumberOfFiles=2
NumberOfLibraries=4
NumberOfModules=1
Version=8.9
@EndOfDesignText@
```

| Key | Meaning |
| --- | --- |
| `AppType` | `StandardJava` for a console (non-UI) program, `JavaFX` for a UI program. |
| `Build<n>` | a build configuration: its name, then the package name (`Default,b4j.example`). |
| `File<n>`, `FileGroup<n>` | a file of the Files folder, and its group in the IDE. |
| `Group` | the main module's group in the IDE. |
| `Library<n>` | a referenced library, lower case (`jcore`, `jfx`, `javaobject`). |
| `Module<n>` | another module of the project: a name in the project folder, or `\|relative\|` and a path to a shared module. |
| `NumberOfFiles`, `NumberOfLibraries`, `NumberOfModules` | the counts of the numbered keys. |
| `Version` | the IDE version that last saved the project. |

### At Level 0

Level 0 runs a console program in one module. A project file is accepted
only if:

- `AppType` is `StandardJava`;
- its only library is jCore (`Library<n>=jcore`, in any case);
- it has no `Module<n>` and no `File<n>`.

Each other `AppType`, library, module and file is an error at its header
line, so all of them are reported at once. A project file with no header, or
no `AppType`, is an error too. The other keys only describe the project in
the IDE and are ignored at Level 0.

`ferro bundle` writes a project's program into the project's directory,
named after the project file: `MyApp/MyApp.b4j` → `MyApp/MyApp`
([cli.md](cli.md#commands)).

## Module file (`.bas`)

```text
B4J=true
Group=Default Group
ModulesStructureVersion=1
Type=StaticCode
Version=10
@EndOfDesignText@
```

| Key | Meaning |
| --- | --- |
| `B4J`, `B4A`, `B4i` | the product that saved the module; no meaning for the code ("Ignore it", see Sources). |
| `Group` | the module's group in the IDE. |
| `ModulesStructureVersion` | the version of the module structure. |
| `Type` | `StaticCode` for a code module, `Class` for a class; B4A also has `Service` and `Activity`. |
| `Version` | the IDE version that last saved the module. |

At Level 0 a module file's `Type` must be `StaticCode`, or the header may be
absent (as in `tests/conformance`). `Type=Class` is an error that points to
Level 1; any other type is an error.

## Sources

- Console projects: [`helloworld.b4j`](https://github.com/rwbl/b4j-howtos/blob/master/helloworld.b4j)
  (B4J 7) and [`Extract Library.b4j`](https://github.com/iranapporg/b4a-extract-library/blob/master/Extract%20Library.b4j)
  (B4J 5.82, keys unsorted, libraries `jcore` and `javaobject`). Both have
  `AppType=StandardJava`, and both programs are console programs.
- A UI project and its modules: the B4X samples added to GitHub Linguist
  ([pull request 5310](https://github.com/github/linguist/pull/5310/files)):
  `BCTextExample.b4j` (`AppType=JavaFX`, modules, files), `.b4a`, `.b4i`,
  and module headers with `Type=Class` and `Type=Service`.
- `Type=StaticCode` is a code module, and the `B4A=true` line has no
  meaning: Erel on the [forum](https://b4x.com/android/forum/threads/b4x-browser-please-provide-example-b4xlibs-to-parse.135060/)
  (posts 6 and 8).

These files are cited, not copied into the repository: their licences are
not checked.
