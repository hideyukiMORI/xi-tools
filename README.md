# xi-tools

English | [日本語](./README.ja.md)

Rust tools written for my own development environment.

**Some of them are not designed as general-purpose tools.** They came out of a personal
environment where I work across dozens of repositories, so some of their assumptions are
tied to that environment. They are public anyway **to keep the problem statements themselves**.

| Tool | What it solves | State |
| --- | --- | --- |
| [`scopegrep`](./scopegrep) | Returns what `grep` does not: **where in the structure a hit belongs** | 🟢 Works (a subset of YAML) |

---

## scopegrep — a grep that knows the structure

`grep` only tells you that a line exists. And **YAML nesting does not show up in line numbers.**

When you look for `cancelled()` in a CI configuration, what you actually want to know is
not "which line it is on" but **"which step it is attached to"**.

Running `grep` over the bundled fixture
([`scopegrep-core/testdata/workflow-with-comment.yml`](./scopegrep-core/testdata/workflow-with-comment.yml))
returns 5 lines.

```console
$ grep -n 'cancelled()' scopegrep-core/testdata/workflow-with-comment.yml
4:#    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。
29:      # 1) 散文。ここに書かれた cancelled() は設定値ではない。
30:      #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。
33:        if: ${{ !cancelled() }}
46:        if: ${{ !cancelled() }}
```

**Three of them (4, 29, 30) are comments, not configuration values.**
The remaining two look alike, but they are different things.

- Line 33 is the `if` of the **dependency audit step** (`Audit (fail on high/critical)`).
  It ran even on a working tree whose earlier step had failed, producing an unintended second red (a defect)
- Line 46 is the `if` of the **Playwright report upload** (`Upload Playwright report`),
  the textbook correct use: uploading the report even when E2E fails

That difference cannot be read out of `grep` output. In fact, on 2026-09-01 a search of this
same shape was misread as "the same defect twice", producing **a false positive (a correct use
judged a defect) and a false negative (a repository whose file was named differently was
missed) at the same time.**

`scopegrep` returns where the matched value belongs in the structure.
**Matches inside comments are not returned by default** (2 lines here, against the 5 above).

```console
$ scopegrep 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

It is **distinguishing comments, not discarding them**, so with `--comments` the same 5 lines
that `grep -n` returns come back, marked with which of the two they were.

```console
$ scopegrep --comments 'cancelled()' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:4: #comment = #    候補パーサは、下の3つの `cancelled()` を **別物として区別できなければならない**。
scopegrep-core/testdata/workflow-with-comment.yml:29: jobs.frontend-check.steps #comment = # 1) 散文。ここに書かれた cancelled() は設定値ではない。
scopegrep-core/testdata/workflow-with-comment.yml:30: jobs.frontend-check.steps #comment = #    !cancelled() を使う理由を説明しているだけで、実行条件ではない。
scopegrep-core/testdata/workflow-with-comment.yml:33: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .if = ${{ !cancelled() }}
scopegrep-core/testdata/workflow-with-comment.yml:46: jobs.e2e.steps[2] "Upload Playwright report" .if = ${{ !cancelled() }}
```

The scope of a comment is decided by **which column it is written at**.
"Whom does this comment explain" is not guessed (lines 29-30 explain `steps[3]`, but an
implementation that holds them in a syntax tree attaches them to `steps[2]`; the measurement
is in "D-2 実測" of the [design note](./docs/design/scopegrep.md)).

🔴 **The `console` blocks in this README are output that was actually produced.**
`scopegrep/tests/readme.rs` runs the commands on every `make check` and checks that they
**match exactly** the lines that follow (if they do not, the test fails).

### Usage

```
scopegrep [-i] [--json] [--comments] [--scope <pattern>] (<needle> | -e <regex>) [<path>...]
```

**`--scope` filters by scope.** Because it queries by **structure** rather than by search
term, it answers questions you cannot write with `grep`, such as "list the `run` of every step"
(with an empty needle, every value at that place is listed).

```console
$ scopegrep --scope '/jobs/*/steps/*/run' '' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:24: jobs.frontend-check.steps[1] "Install" .run = npm ci
scopegrep-core/testdata/workflow-with-comment.yml:27: jobs.frontend-check.steps[2] "Unit tests" .run = npm test
scopegrep-core/testdata/workflow-with-comment.yml:34: jobs.frontend-check.steps[3] "Audit (fail on high/critical)" .run = npm audit --audit-level=high
scopegrep-core/testdata/workflow-with-comment.yml:42: jobs.e2e.steps[1] "Run Playwright" .run = npx playwright test
```

Patterns are written in the same JSON Pointer form as the output. `*` is **exactly one
segment**, `**` is **zero or more** (`/services/**/image` hits an `image` at any depth).
Everything else is an exact match against the raw key or index, and **there is no substring
glob** (to keep `*` with a single meaning). The match is against the **whole** scope path.
An unreadable pattern is not silently repaired: it says why and exits with code 2.

- **`-i` / `--ignore-case`** — matches ignoring case. The column stays at
  **the position of the match in the original text** (counting on a lowercased string would
  shift the column only on lines containing a character whose lowercase form is two
  characters, such as `İ`)
- **`-e` / `--regex`** — searches with a regular expression instead of a fixed string.
  It is exclusive with `<needle>`; when it is given, every positional argument becomes a
  `<path>` ([using it requires a build-time flag](#install))
- **Omitting the path** — `scopegrep <needle>` alone recurses from where you are.
  The display carries no `./`. It does when `.` is passed explicitly (same as `grep -rn x .`)
- **Dependency directories are not descended into** — `.git` `node_modules` `vendor` `target` `.venv`.
  In a local measurement (2026-09-02) there were 188 of my own `.yml` / `.yaml` files, against
  3,206 under `node_modules` and 3,837 under `vendor`; descending into them makes almost all
  of the output someone else's files. **A path that is named explicitly is not excluded**
  (`scopegrep x node_modules/foo/` is read)

### Regular expressions are opt-in

`-e` / `--regex` is **not in the default build**. This tool assumes "shippable as a single
binary, with a core that has zero dependencies", so the `regex` crate (3 crates transitively)
is handed only to those who need it
(the reasoning is in [ADR 0002](./docs/adr/0002-regex-is-an-opt-in-feature.md)).
It works independently of `--scope`, which filters by scope.

```console
$ scopegrep -e 'npm (ci|test)' scopegrep-core/testdata/
scopegrep-core/testdata/workflow-with-comment.yml:24: jobs.frontend-check.steps[1] "Install" .run = npm ci
scopegrep-core/testdata/workflow-with-comment.yml:27: jobs.frontend-check.steps[2] "Unit tests" .run = npm test
```

A match is **within one line** (`^` and `$` are the start and end of a line, and a match does
not span a multi-line scalar). That follows from a design that holds values line by line.
`-i` is passed to the regular expression side as `RegexBuilder::case_insensitive`, so Unicode
is handled slightly differently from the character-by-character case folding of the
fixed-string side.

🔴 **Running `-e` on a binary built without regular expressions exits with code 2 and says
that this binary was built without regular expressions.**
It does not silently fall back to a fixed string. Which build you have is reported by
`scopegrep --version` as `(regex: on)` / `(regex: off)`.

### Install

```bash
cargo install scopegrep                    # fixed-string search, zero dependencies
cargo install scopegrep --features regex   # adds -e/--regex (3 crates: regex, regex-automata, regex-syntax)
```

Binaries for each OS (with regular expressions) are on
[GitHub Releases](https://github.com/hideyukiMORI/xi-tools/releases).

### Design decisions

- **Matches inside comments are not returned by default.** It reads the structure rather than
  lines, so prose such as `# ... cancelled() ...` is not confused with an actual configuration
  value. A line-based search always picks those up (that is the difference between the 5 lines
  and the 2 lines above). The result of that distinction is not thrown away: with `--comments`
  they come back **explicitly marked as comments**
- **Only a subset of YAML is read, and everything outside it is an error.**
  What can be read is block mappings, block sequences, single-line scalars
  (plain / `'…'` / `"…"`), block scalars (`|` `>`), flow notation (multi-line too; it is not
  entered, and is held as a value line by line), tags (skipped), comments, and a leading `---`.
  Anchors, aliases, merge keys, multi-line plain scalars and multiple documents **cannot be
  read**. Rather than silently returning a misread result, it **says what on which line it
  could not read, and fails**
  (the list is under "対応する YAML の部分集合" in the [design note](./docs/design/scopegrep.md))
- **The subset grows by measurement.** Run against the 188 `.yml` / `.yaml` files across every
  repository on this machine, v1 read 169 of them, and read all 67 GitHub Actions workflows.
  Of the 18 it could not read, 14 were compose `healthcheck.test` split across lines in flow
  notation, 3 were the compose `!override` / `!reset` tags, and 1 was a **bug** that misread
  `- { $ref: … }`. v1.1, which added only those three, reached 187 / 188, and the remaining
  one is a fixture of my own that exists to confirm what cannot be read. Anchors and multiple
  documents occurred 0 times in those 188 files, so they are still not read
  (the numbers come from "実ファイルでの計測" in the design note)
- **It has machine-readable output.** `--json` is JSON Lines, one hit per line, and returns
  the scope as an RFC 6901 JSON Pointer as well. `kind` is always present, with or without
  `--comments` (if the set of keys changed with the input, the receiver could not tell it
  apart from "it just did not come out this time")

```console
$ scopegrep --json 'cancelled()' scopegrep-core/testdata/workflow-with-comment.yml
{"file":"scopegrep-core/testdata/workflow-with-comment.yml","line":33,"column":18,"pointer":"/jobs/frontend-check/steps/3/if","path":"jobs.frontend-check.steps[3] \"Audit (fail on high/critical)\" .if","label":"Audit (fail on high/critical)","value":"${{ !cancelled() }}","kind":"value"}
{"file":"scopegrep-core/testdata/workflow-with-comment.yml","line":46,"column":18,"pointer":"/jobs/e2e/steps/2/if","path":"jobs.e2e.steps[2] \"Upload Playwright report\" .if","label":"Upload Playwright report","value":"${{ !cancelled() }}","kind":"value"}
```

- **Exit codes are the same as `grep`.** 0 = at least one hit / 1 = no hit / 2 = error.
  🔴 **If there is a file it could not read, it exits with 2 even when there were hits.**
  Not calling "a result that looked at only part of the input" a success is this tool's answer
  to the accident it was born from
- **The default build has zero dependencies.** The core (`scopegrep-core`) is written as
  `#![no_std]` plus `alloc`, and **cannot syntactically reach** the clock, randomness, the
  environment or I/O. Why the parser is hand-written, and the measurement of 6 candidates
  (position information, exposure of comments, number of dependencies, `no_std`), are in the
  "D-2 実測" section of the [design note](./docs/design/scopegrep.md).
  The one exception is the opt-in `regex`, which goes **into the binary, not into the core**
  (the core only receives matching through a `Matcher` trait, and knows nothing about regular
  expressions). Licenses, vulnerabilities, duplicate versions and sources are watched by
  `make deny` (`cargo-deny`)
- **It is not limited to YAML.** The same problem exists in TOML / JSON as well
  (they are not in v1)

### Adjacent existing implementations

- [`yamlpath`](https://crates.io/crates/yamlpath) — a library that extracts values from YAML
  (preserving the formatting). It goes in the direction of **"give a path, get a value"**,
  the opposite of `scopegrep`'s **"search for a value, get a path"**
- [`treegrep`](https://crates.io/crates/treegrep) — displays search results as a **file tree**.
  What it returns is the file hierarchy, **not the structure inside a file**

---

## Development

```bash
make check
```

**This is the only entry point.** CI also does nothing but call `make check`, so that no check
exists only in CI (to structurally prevent "it passed locally but failed in CI").
Only two tools are needed — `make coverage` needs `cargo-llvm-cov` and `make deny` needs
`cargo-deny` (both `cargo install <name> --locked`).

`make check` runs lint and tests in **both configurations** (the default and
`--features scopegrep/regex`), so that a state where only one of them is green cannot arise.

The toolchain version is decided by `rust-toolchain.toml`. It is written neither in the
`Makefile` nor in CI (writing it in two places lets one of them be bumped alone, which creates
"it passes locally").

### Rules

How code is written is **defined by [`docs/coding-rules.md`](./docs/coding-rules.md)**.
Every rule has an ID, and **the state of its mechanical enforcement is stated explicitly**
(active / planned / impossible / not adopted).

The idea is to **fix a single way of expressing a single thing**, and it is implemented in
three layers.

| Layer | What it guards |
| --- | --- |
| compiler / cargo | types, visibility, exhaustiveness, crate boundaries (**making invalid states unwritable**) |
| lint | what can be written but should not be |
| conformance check (`xtask`) | rules specific to xi-tools (zero dependencies, part of `make check`) |

🔴 **Suppression has two stages.** A rule that is `forbid`den makes both `#[allow]` and
`#[expect]` a compile error (E0453), so **there is no window through which to apply for an
exception**. A `deny` rule can be suppressed only with `#[expect(lint, reason = "...")]`, and
a suppression that is no longer needed is failed by `unfulfilled_lint_expectations`.

The reasoning is in
[ADR 0001](./docs/adr/0001-strictness-is-mechanically-enforced.md), and the measurement that
the gates actually fire is in
[the proof of gate firing](./docs/quality/gate-proofs.md).

## License

MIT
