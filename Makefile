.PHONY: phony
phony:

build:
	@cargo build

run:
	@cargo run

release:
	@cargo build --release

test: phony
	@cargo check
	@cargo test --all
	@\
	cd test/cdk && \
	$(MAKE) install diff deploy

test-reset:
	@\
	cd test/cdk && \
	$(MAKE) DESTROY && \
	ACCOUNT=$$(aws sts get-caller-identity --query "Account" --output text) && \
	if aws s3api head-bucket --bucket "$${ACCOUNT}-migration-test-1" 2>/dev/null; then \
		aws s3api delete-bucket --bucket $${ACCOUNT}-migration-test-1; \
	fi;\
	if aws s3api head-bucket --bucket "$${ACCOUNT}-migration-test-2" 2>/dev/null; then \
		aws s3api delete-bucket --bucket $${ACCOUNT}-migration-test-2; \
	fi;

lint:
	@cargo fmt -- --check
	@cargo clippy -- -D warnings
