# xi-tools

English | [日本語](./README.ja.md)

Rust tools written for my own development environment.

**Some of them are not designed as general-purpose tools.** They came out of a personal
environment where I work across dozens of repositories, so some of their assumptions are
tied to that environment. They are public anyway **to keep the problem statements themselves**.

| Tool | What it solves | State |
| --- | --- | --- |
| [`scopegrep`](./scopegrep) | Returns what `grep` does not: **where in the structure a hit belongs** | 🟢 Works (a subset of YAML) |
| [`fleet-top`](./fleet-top) | **The state of dozens of repositories on one screen** — branch, dirty, ahead/behind, open PRs, CI, stale branches — in the time a command still gets run | 🟢 Works (60 repositories in 1.6–1.8 s, measured) |

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

## fleet-top — the state of dozens of repositories on one screen

I work across about 60 git repositories side by side. Several times a day the question is the
same: **for every one of them, which branch am I on, is there anything uncommitted, how far is
it from its upstream, are there open PRs, is CI green, are there stale branches?**
Every time, that turned into a throwaway shell loop — and the loop was slow.

Measured on 2026-09-01: one `gh api` call takes 0.67–0.74 s. Three calls per repository
(settings, open PRs, CI) over 42 repositories is 126 calls, or **about 84–93 seconds** in series.

🔴 **A command that takes 84 seconds does not get run.** And because it does not get run, the
things it would have shown — an expired deadline, an audit that nobody looked at for four
days — stay unseen. The goal of this tool is not "fast and pleasant"; it is **to cross the line
between a command that gets run and one that does not**.

```text
$ fleet-top ~/docker
REPO                                  BRANCH                                  DIRTY  AHEAD/BEHIND  PR   CI    STALE
NENE2                                 main                                    -      -             10   ok    -
NENE2-examples-repo                   main                                    -      -             -    -     -
NeNe                                  main                                    -      -             -    ok    ?
_work                                 main                                    -      -             -    -     -
eventlog                              docs/ft13-milestone                     -      (none)        n/a  n/a   n/a
gtypist-lesson                        master                                  -      -             -    -     -
hideyuki-mori-site                    main                                    -      -             -    -     -
hideyukiMORI                          master                                  1      (none)        n/a  n/a   n/a
hoplog                                main                                    -      -             -    -     -
keyquest                              main                                    -      -             -    -     -
knowledgelog                          main                                    -      (none)        n/a  n/a   n/a
…（49 more rows）
fleet-top: NeNe: more than 100 branches; STALE was not counted
fleet-top: 60 repos, 45 on GitHub, 1.6s
```

Captured on 2026-09-02 against my own directory (60 rows, first 11 shown; the last two lines
are stderr). Unlike the `scopegrep` examples above, **this block is not verified by a test** —
the output depends on the state of GitHub and of my working trees at that moment. What is
verified is the formatting: the fixture tests in `fleet-top-core` check the table character
by character.

Reading the table:

- `-` is zero or nothing to report. `n/a` is a repository whose `origin` is not GitHub (it was
  not asked). `?` is a value that could not be determined — **each `?` comes with one line on
  stderr saying why**, and the row is kept. Deleting a row that failed is the same shape as the
  accident this tool was written to prevent (judging from the half you happened to see)
- The exit code is 0 when every row is complete, 1 when any `?` is present (the table was still
  printed), 2 for a usage error or an unreadable directory
- `AHEAD/BEHIND` is the difference to the tracking branch as it is on disk. The tool **never
  runs `git fetch`** — it only looks

### Usage

```
fleet-top [DIR] [--stale-days N] [--no-github]
```

`DIR` defaults to `.`. Only its direct children that contain a `.git` are repositories; there is
no recursion. `--stale-days` (default 30) is the age after which a non-default branch on GitHub
counts as stale. `--no-github` never starts `gh` and prints `n/a` in the three GitHub columns.

GitHub is read through `gh api graphql`, so **`gh` must be installed and logged in**; the tool
borrows its authentication and handles no token of its own.

### Install

```bash
cargo install fleet-top
```

Binaries for each OS are on
[GitHub Releases](https://github.com/hideyukiMORI/xi-tools/releases) (tag `fleet-top-v…`).

### What was measured before writing it

The design was decided by a one-hour prototype, not by expectation
(the full table is in [`docs/benchmarks/fleet-top.md`](./docs/benchmarks/fleet-top.md)).

| Approach | 60 repositories |
| --- | --- |
| REST, in series (extrapolated from 21 calls at 0.74 s) | 93 s |
| REST, 64 calls in parallel | 2.38 s, 126 rate-limit points |
| **One** GraphQL request carrying all repositories | 8.87 s for 42 repositories; **HTTP 502** for 60 |
| GraphQL, **3 repositories per request, all requests in parallel** | **1.35–1.49 s**, 20 points |
| The tool itself, release build, 45 of 60 repositories on GitHub | **1.6–1.8 s** |

The result I did not expect: **GraphQL gets slower the more repositories you put in one
request**, and 60 in one request does not come back at all. Splitting into small requests and
sending them all at once was faster than either the single request or REST at any parallelism.
`REPOS_PER_QUERY = 3` in the core comes straight from that table.

### Failures that actually happened

- **`gh api graphql` exits 1 as soon as one repository in the request fails**, with the other
  repositories' data still on stdout. Judging by exit code would have dropped three
  repositories for one that was missing. The tool reads stdout regardless of the exit code and
  fails only the repository named in `errors[].path`
- **The design note's own example contradicted its own rules** (rows out of byte order; a row
  that was "unreadable" yet had a branch name). The implementation's exact-match fixture test
  caught it, and the note was corrected
- **The first real run printed a `?` with no reason.** A repository with more than 100
  branches has its stale count truncated — that is not a failure, so nothing was written to
  stderr, but it was still a `?`. Now it says so, and a test covers it

### Design decisions

Recorded with the rejected alternatives in [`docs/design/fleet-top.md`](./docs/design/fleet-top.md)
and [ADR 0003](./docs/adr/0003-fleet-top-fetches-github-via-chunked-graphql.md).

- **Zero dependencies, in both crates.** GitHub is read through the `gh` subprocess (its
  authentication is borrowed), concurrency is `std::thread::scope` with a bounded worker pool,
  and the JSON of the GraphQL response is read by a hand-written RFC 8259 parser in
  `fleet-top-core`. `serde_json` would have brought 5 crates and broken the `no_std` core
- **The core is `no_std` even though the tool is all I/O.** It receives the output of `git`
  and `gh` as strings and "today" as a value, and returns the table; starting processes,
  waiting for them and reading the clock stay in the binary. Everything the core does is
  tested from fixtures alone
- **No TUI, no `--watch`.** A tool that returns in 1.6 s has no reason to stay resident
  (`watch fleet-top` exists). Adding `ratatui` would have added dozens of crates
- **Read-only.** No `fetch`, no `checkout`, no `merge`. It only looks

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
