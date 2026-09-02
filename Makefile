# xi-tools のビルドと品質ゲート。
#
# 🔴 `make check` がこのリポジトリの唯一の正である（QLT-003）。
#    CI は check を呼ぶだけで、CI 側にだけ存在する検査を作らない。
#    「ローカルでは通ったのに CI で落ちた」を構造的に起こさないため。
#
# 🔴 道具の版は rust-toolchain.toml が決める。ここにも CI にも版を書かない。
#    2箇所に書くと、片方だけ上げられて「手元では通る」が生まれる。
CARGO ?= cargo

.PHONY: all check fmt fmt-check lint test conformance coverage deny build doc-check clean prove package check-version tag-tool tag-version release-features

## 🔴 opt-in の feature（ADR 0002）。**片方だけ緑の状態を作らない**ので、
##    lint と test は既定構成と FEATURES 構成の両方で走らせる。
FEATURES ?= scopegrep/regex

all: check

## check — 提出前に必ず通すもの。CI もこれを呼ぶ
check: fmt-check lint test conformance coverage deny doc-check build

## fmt — rustfmt に設定ファイルを置かない。整形の流儀を議論する余地をそもそも作らない
fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all --check

## lint — 規則との対応は docs/coding-rules.md
## 🔴 --all-targets を外さないこと。外すとテストコードが検査対象から落ちる。
## 🔴 --locked は Cargo.lock が更新される状態でのゲート通過を拒む（QLT-004）。
## 🔴 2回目（--features）を CI ではなくここに置く。CI 側にだけ検査を作らない（QLT-003）。
lint:
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings
	$(CARGO) clippy --workspace --all-targets --locked --features $(FEATURES) -- -D warnings

## test — 🔴 --locked は同上。両構成で回す（ADR 0002 決定 5）。
test:
	$(CARGO) test --workspace --locked
	$(CARGO) test --workspace --locked --features $(FEATURES)

## conformance — このリポジトリ固有の規約検査（CNF-0xx / xtask）
## lint が見ないものだけを見る。規則の正本は docs/coding-rules.md。
conformance:
	$(CARGO) run --quiet -p xtask --locked

## coverage — 行カバレッジの下限（QLT-008）。上げる方向にしか動かさない。
## 🔴 下限は実測（2026-09-02: 92.21%）より下に置いた。100% を目標にするための数字ではなく、
##    「テストを消したら落ちる」ための数字である。
## cargo-llvm-cov 0.9.0 は下限割れをメッセージ無しの終了コード 1 で返す（実測）。
## 導入: rustup component add llvm-tools-preview && cargo install cargo-llvm-cov --locked
COVERAGE_MIN_LINES ?= 90
coverage:
	@command -v cargo-llvm-cov >/dev/null || { echo "coverage: cargo-llvm-cov が無い。cargo install cargo-llvm-cov --locked"; exit 2; }
	$(CARGO) llvm-cov --workspace --locked --features $(FEATURES) --summary-only --fail-under-lines $(COVERAGE_MIN_LINES)

## deny — 依存の許可制（ARC-004 / ADR 0002）。方針の正本は deny.toml。
## 🔴 --deny license-not-encountered を外さないこと。外すと deny.toml の allow に
##    使っていないライセンスを書き足せてしまい、依存が増えたときに気づけなくなる。
## 導入: cargo install cargo-deny --locked
deny:
	@command -v cargo-deny >/dev/null || { echo "deny: cargo-deny が無い。cargo install cargo-deny --locked"; exit 2; }
	$(CARGO) deny --locked check --deny license-not-encountered

## doc-check — rustdoc の警告（壊れた intra-doc link 等）を失敗にする
doc-check:
	RUSTDOCFLAGS="-D warnings" $(CARGO) doc --workspace --no-deps --locked

build:
	$(CARGO) build --workspace --release --locked

## prove — ゲートが実際に発火することを確かめる（QLT-006）。
## 検査の最大の失敗は、見逃したまま常に緑を返すことである。本物のコードを
## 見ている限りそれは発覚しない。手順と結果は docs/quality/gate-proofs.md。
prove:
	@echo "docs/quality/gate-proofs.md の手順に従って手で実行する"

## tag-tool / tag-version — タグをツール名と版に分ける（release.yml が呼ぶ）。
##
## タグは `<tool>-vX.Y.Z`（例: fleet-top-v0.1.0・scopegrep-v0.2.0）。
## 🔑 `vX.Y.Z` だけの形は scopegrep として受ける。v0.1.0 が接頭辞の無い時代に打たれていて、
##    打ち直せない（publish 済みの版のタグは打ち直さない・docs/release.md 8）。
## 🔴 ツールごとに版を持つのは ARC-001（1 ツール = 1 クレート・単独 publish）の帰結である。
##    workspace 全体で版を揃えると、変えていないツールに空の版が出る。
tag-tool:
	@case "$(TAG)" in \
	  "") echo "tag-tool: TAG= が空。例: TAG=fleet-top-v0.1.0" >&2; exit 2 ;; \
	  v?*) echo scopegrep ;; \
	  *-v?*) echo "$(TAG)" | sed 's/-v[^-]*$$//' ;; \
	  *) echo "tag-tool: TAG=$(TAG) は <tool>-vX.Y.Z の形でない" >&2; exit 2 ;; \
	esac

tag-version:
	@case "$(TAG)" in \
	  v?*|*-v?*) echo "$(TAG)" | sed 's/^.*v//' ;; \
	  *) echo "tag-version: TAG=$(TAG) は <tool>-vX.Y.Z の形でない" >&2; exit 2 ;; \
	esac

## release-features — 配る binary に付ける feature（release.yml が呼ぶ）。
## 🔴 scopegrep だけ regex 入り（ADR 0002）。既定ビルドが依存 0 であることと、配布物に
##    正規表現が入っていることは両立する。他のツールは feature を持たない。
release-features:
	@case "$(TOOL)" in \
	  "") echo "release-features: TOOL= が空" >&2; exit 2 ;; \
	  scopegrep) echo "--features regex" ;; \
	  *) echo "" ;; \
	esac

## check-version — タグと Cargo.toml の版が一致することを確かめる（release.yml が呼ぶ）。
##
## 🔴 版の正本は Cargo.toml である。タグを打ち間違えると、タグ名と中身の版が違う
##    binary が Release に並ぶ。それを人の注意ではなく、ここで落とす。
## 🔴 判定を workflow 側に書かないこと（QLT-003）。手元で `make check-version TAG=fleet-top-v0.1.0`
##    と打って、CI と同じ判定が同じ言葉で返ることに意味がある。
##
## 🔑 依存宣言（bin の `<tool>-core = { ..., version = "x" }`）も同じ版か見る。
##    ここがずれると、単独 publish した bin が古い core を引く。
check-version:
	@tool=`$(MAKE) -s tag-tool TAG="$(TAG)"` || exit 2; \
	expected=`$(MAKE) -s tag-version TAG="$(TAG)"` || exit 2; \
	if [ ! -f "$$tool/Cargo.toml" ]; then \
	  echo "check-version: タグ $(TAG) が指すツール $$tool が無い（$$tool/Cargo.toml が見つからない）"; exit 2; \
	fi; \
	bin=`sed -n 's/^version = "\([^"]*\)".*/\1/p' "$$tool/Cargo.toml" | head -n 1`; \
	core=`sed -n 's/^version = "\([^"]*\)".*/\1/p' "$$tool-core/Cargo.toml" | head -n 1`; \
	dep=`sed -n "s/^$$tool-core = .*version = \"\([^\"]*\)\".*/\1/p" "$$tool/Cargo.toml" | head -n 1`; \
	status=0; \
	for pair in "$$tool の版:$$bin" "$$tool-core の版:$$core" "$$tool の依存宣言:$$dep"; do \
	  found="$${pair#*:}"; \
	  if [ "$$found" != "$$expected" ]; then \
	    echo "check-version: タグ $(TAG) に対し $${pair%%:*}が $$found（期待: $$expected）"; \
	    status=1; \
	  fi; \
	done; \
	if [ $$status -ne 0 ]; then \
	  echo "check-version: 版の正本は Cargo.toml。タグではなく Cargo.toml を直すか、タグを打ち直す"; \
	  exit 1; \
	fi; \
	echo "check-version: タグ $(TAG) と $$tool の Cargo.toml の版 $$expected は一致する"

## package — crates.io に出す .crate を作れることを確かめる（手順は docs/release.md）。
##
## 🔴 make check には入れない。ネットワークと時間を要するので、
##    「提出前に必ず通すもの」と「配る直前に1回やること」を混ぜない。
##
## 🔴 **2つを1回の cargo package で作ること**（2026-09-02 実測）。
##    分けて `cargo package -p scopegrep` を打つと、--no-verify を付けても
##    「no matching package named `scopegrep-core` found」で落ちる。
##    .crate に入れる Cargo.lock を解決する時点で crates.io を見にいくためで、
##    まだ core が公開されていない初回は原理的に通らない。
##    まとめて渡すと、cargo が一時レジストリに core を置いて bin を検証する
##    （実測: "Unpacking scopegrep-core (registry .../tmp-registry)"）。
##    ⇒ 初回でも検証ビルドまで通る。--no-verify で目をつぶる必要がない。
##
## 🔑 作業ツリーが汚れていると cargo が拒む。commit してから打つこと
##    （--allow-dirty で黙らせない。何を配ったかが git で辿れなくなる）。
## 🔑 TOOL= で出すツールを選ぶ（既定 scopegrep）。`make package TOOL=fleet-top`。
TOOL ?= scopegrep
package:
	$(CARGO) package -p $(TOOL)-core -p $(TOOL) --locked

clean:
	$(CARGO) clean
