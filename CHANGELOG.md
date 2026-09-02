# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project follows [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] — 2026-09-02

First release of `scopegrep` (the command) and `scopegrep-core` (its `no_std` core).

### Added

- `scopegrep <needle> [<path>...]` — searches a subset of YAML and prints, per hit, the file,
  the line and the scope the matched value belongs to. Exit codes are those of `grep`:
  0 = at least one hit, 1 = no hit, 2 = error. **A file that could not be read makes it exit
  with 2 even when there were hits**
- Matches inside comments are not returned by default. `--comments` returns them marked as
  comments, which gives back the same lines as `grep -n`. The scope of a comment is decided by
  the column it is written at; whom it explains is not guessed
- `--scope <pattern>` — filters by structure rather than by search term, in the same JSON
  Pointer form as the output. `*` is exactly one segment, `**` is zero or more, and there is no
  substring glob. The match is against the whole scope path, and an unreadable pattern exits
  with code 2
- `-i` / `--ignore-case` — the reported column stays at the position of the match in the
  original text
- `--json` — JSON Lines, one hit per line, carrying the scope as an RFC 6901 JSON Pointer.
  `kind` is present with or without `--comments`
- The path may be omitted, in which case the current directory is walked recursively.
  `.git`, `node_modules`, `vendor`, `target` and `.venv` are not descended into; a path that is
  named explicitly is not excluded
- `-e` / `--regex` behind the `regex` feature, which is off by default. The default build has
  zero dependencies; with the feature there are 3 crates (`regex`, `regex-automata`,
  `regex-syntax`). Without the feature, `-e` exits with code 2 and says that the binary was
  built without regular expressions instead of falling back to a fixed string.
  `scopegrep --version` reports `(regex: on)` / `(regex: off)`
- The YAML subset that is read: block mappings, block sequences, single-line scalars
  (plain, `'…'`, `"…"`), block scalars (`|`, `>`), flow notation including multi-line, tags,
  comments, and a leading `---`. Anchors, aliases, merge keys, multi-line plain scalars and
  multiple documents are not read; they are reported with their kind and line number instead of
  being misread
- `scopegrep-core` — `#![no_std]` plus `alloc`, with zero dependencies, written by hand after
  measuring 6 candidate parsers on the same fixture (position information, exposure of comments,
  number of dependencies, `no_std`). Matching is received through a `Matcher` trait, so the core
  does not know about regular expressions in either configuration
- The subset was decided by measurement, not by guessing: over the 188 `.yml` / `.yaml` files on
  the author's machine, v1 read 169, including all 67 GitHub Actions workflows. Of the 18 that
  could not be read, 14 were multi-line flow notation, 3 were tags and 1 was a misreading of
  `- { $ref: … }`. Adding those three cases brought it to 187 / 188, the remaining file being a
  fixture that exists to confirm what cannot be read. Anchors and multiple documents occurred
  0 times in those files and are therefore still not read
- The examples in `README.md` and `README.ja.md` are executed and compared on every
  `make check`, so an example that does not work fails the build
- Coding rules that are enforced mechanically rather than documented: `docs/coding-rules.md` is
  the source of truth, and the enforcement lives in `Cargo.toml` (`forbid` / `deny` lints),
  `clippy.toml`, the `Makefile` and `xtask`. A suppression must cite a rule ID that exists in
  `docs/coding-rules.md`, and `xtask` fails if it does not
- `make check` as the single entry point, which CI does nothing but call: format, lint, tests,
  conformance, coverage (a lower bound of 90% lines), `cargo-deny` and rustdoc, in both the
  default and the `regex` configuration

[0.1.0]: https://github.com/hideyukiMORI/xi-tools/releases/tag/v0.1.0
