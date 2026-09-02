# xi-tools のビルドと品質ゲート。
#
# 🔴 `make check` がこのリポジトリの唯一の正である（QLT-003）。
#    CI は check を呼ぶだけで、CI 側にだけ存在する検査を作らない。
#    「ローカルでは通ったのに CI で落ちた」を構造的に起こさないため。
#
# 🔴 道具の版は rust-toolchain.toml が決める。ここにも CI にも版を書かない。
#    2箇所に書くと、片方だけ上げられて「手元では通る」が生まれる。
CARGO ?= cargo

.PHONY: all check fmt fmt-check lint test conformance coverage deny build doc-check clean prove package check-version

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

## check-version — タグと Cargo.toml の版が一致することを確かめる（release.yml が呼ぶ）。
##
## 🔴 版の正本は Cargo.toml である。タグを打ち間違えると、タグ名と中身の版が違う
##    binary が Release に並ぶ。それを人の注意ではなく、ここで落とす。
## 🔴 判定を workflow 側に書かないこと（QLT-003）。手元で `make check-version TAG=v0.1.0`
##    と打って、CI と同じ判定が同じ言葉で返ることに意味がある。
##
## 🔑 依存宣言（scopegrep の `scopegrep-core = { ..., version = "x" }`）も同じ版か見る。
##    ここがずれると、単独 publish した bin が古い core を引く。
check-version:
	@case "$(TAG)" in \
	  v?*) ;; \
	  "") echo "check-version: TAG= が空。例: make check-version TAG=v0.1.0"; exit 2 ;; \
	  *) echo "check-version: TAG=$(TAG) は v で始まっていない。タグは v0.1.0 の形で打つ"; exit 2 ;; \
	esac
	@expected="$(TAG)"; expected="$${expected#v}"; \
	bin=`sed -n 's/^version = "\([^"]*\)".*/\1/p' scopegrep/Cargo.toml | head -n 1`; \
	core=`sed -n 's/^version = "\([^"]*\)".*/\1/p' scopegrep-core/Cargo.toml | head -n 1`; \
	dep=`sed -n 's/^scopegrep-core = .*version = "\([^"]*\)".*/\1/p' scopegrep/Cargo.toml | head -n 1`; \
	status=0; \
	for pair in "scopegrep の版:$$bin" "scopegrep-core の版:$$core" "scopegrep の依存宣言:$$dep"; do \
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
	echo "check-version: タグ $(TAG) と Cargo.toml の版 $$expected は一致する"

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
package:
	$(CARGO) package -p scopegrep-core -p scopegrep --locked

clean:
	$(CARGO) clean
