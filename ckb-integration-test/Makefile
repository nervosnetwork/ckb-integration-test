CKB_GITHUB_REPOSITORY := ${CKB_GITHUB_REPOSITORY}
CKB_GITHUB_SHA := ${CKB_GITHUB_SHA}

##@ Building

.PHONY: build
build: ## Build CKB-Integration-Test binary.
	cargo build --release

.PHONY: build-ckb
build-ckb: ## Build CKB binary.
	[ -d repo/${CKB_GITHUB_REPOSITORY} ] || git clone https://github.com/${CKB_GITHUB_REPOSITORY} repo/${CKB_GITHUB_REPOSITORY}
	cd repo/${CKB_GITHUB_REPOSITORY} && git checkout ${CKB_GITHUB_SHA} && make build

##@ Help

.PHONY: help
help:  ## Display help message.
	@awk 'BEGIN {FS = ":.*##"; printf "\nUsage:\n  make \033[36m<target>\033[0m\n"} /^[a-zA-Z_-]+:.*?##/ { printf "  \033[36m%-15s\033[0m %s\n", $$1, $$2 } /^##@/ { printf "\n\033[1m%s\033[0m\n", substr($$0, 5) } ' $(MAKEFILE_LIST)
