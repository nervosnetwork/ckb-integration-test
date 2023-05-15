.PHONY: all
all: ckb-testkit ckb-bench

.PHONY: ckb-testkit
ckb-testkit:
	cd ckb-testkit && cargo build

.PHONY: ckb-bench
ckb-bench:
	cd ckb-bench && cargo build

.PHONY: check
check:
	cd ckb-testkit && cargo check
	cd ckb-bench && cargo check
	cd ckb-integration-test && cargo check

.PHONY: fmt
fmt:
	cd ckb-testkit && cargo fmt --all
	cd ckb-bench && cargo fmt --all
