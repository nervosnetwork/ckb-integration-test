.PHONY: all
all: ckb-testkit ckb-integration-test ckb-bench

.PHONY: ckb-testkit
ckb-testkit:
	cd ckb-testkit && cargo build

.PHONY: ckb-integration-test
ckb-integration-test:
	cd ckb-integration-test && cargo build

.PHONY: ckb-bench
ckb-bench:
	cd ckb-bench && cargo build

.PHONY: fmt
fmt:
	cd ckb-testkit && cargo fmt --all
	cd ckb-integration-test && cargo fmt --all
	cd ckb-bench && cargo fmt --all
