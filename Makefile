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

# @TODO: complete test target. It should do the following:
# - test migration from stack A to B, both buckets separately
# - cdk diff should report changes
# - migrate both resources back from stac k B to A
# - cdk diff should report no changes
# - cdk DESTROY


# @TODO: add target to lookup supported resources on https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/resource-import-supported-resources.html and update the file src/supported_resource_types.rs
