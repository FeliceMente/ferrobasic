# Real-world corpus

Open source B4X projects used to measure how many real projects FerroBASIC can
parse, type-check, build and run unchanged.

Rules:

- Only add projects whose licence allows redistribution. Record the source URL,
  licence and commit in `corpus.toml` next to each project.
- Never add third-party libraries (`.jar`, `.xml`, closed `.b4xlib`) or B4X binaries.
- Projects that cannot be redistributed can still be listed by URL in
  `corpus.toml` and fetched locally by the test tooling.

Layout (to be created):

```text
tests/corpus/
  corpus.toml          list of projects: name, url, licence, commit, product
  <project-name>/      the project as published, unchanged
```
