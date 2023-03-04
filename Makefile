build:
	@cargo build

run:
	@cargo run

release:
	@cargo build --release

# @TODO: add target to run tests

# @TODO: add target to lookup supported resources on https://docs.aws.amazon.com/AWSCloudFormation/latest/UserGuide/resource-import-supported-resources.html and update the file src/supported_resource_types.rs
