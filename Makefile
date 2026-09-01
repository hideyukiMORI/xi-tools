# xi-tools のビルドと品質ゲート。
#
# 🔴 `make check` がこのリポジトリの唯一の正である（QLT-003）。
#    CI は check を呼ぶだけで、CI 側にだけ存在する検査を作らない。
#    「ローカルでは通ったのに CI で落ちた」を構造的に起こさないため。
#
# 🔴 道具の版は rust-toolchain.toml が決める。ここにも CI にも版を書かない。
#    2箇所に書くと、片方だけ上げられて「手元では通る」が生まれる。
CARGO ?= cargo

.PHONY: all check fmt fmt-check lint test conformance build doc-check clean prove

all: check

## check — 提出前に必ず通すもの。CI もこれを呼ぶ
check: fmt-check lint test conformance doc-check build

## fmt — rustfmt に設定ファイルを置かない。整形の流儀を議論する余地をそもそも作らない
fmt:
	$(CARGO) fmt --all

fmt-check:
	$(CARGO) fmt --all --check

## lint — 規則との対応は docs/coding-rules.md
## 🔴 --all-targets を外さないこと。外すとテストコードが検査対象から落ちる。
## 🔴 --locked は Cargo.lock が更新される状態でのゲート通過を拒む（QLT-004）。
lint:
	$(CARGO) clippy --workspace --all-targets --locked -- -D warnings

## test — 🔴 --locked は同上
test:
	$(CARGO) test --workspace --locked

## conformance — このリポジトリ固有の規約検査（CNF-0xx / xtask）
## lint が見ないものだけを見る。規則の正本は docs/coding-rules.md。
conformance:
	$(CARGO) run --quiet -p xtask --locked

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

clean:
	$(CARGO) clean
