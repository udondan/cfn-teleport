.PHONY: phony
phony:

build:
	@cargo build

run:
	@cargo run

release:
	@cargo build --release

test: phony
	@\
	cd test/cdk && \
	$(MAKE) install diff deploy

test-reset:
	@\
	cd test/cdk && \
	$(MAKE) DESTROY
