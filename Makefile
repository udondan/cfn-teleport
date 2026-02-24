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
	cd test/cloudformation && \
	$(MAKE) deploy

test-clean-all:
	@\
	set -x; \
	aws resourcegroupstaggingapi get-resources \
			--tag-filters Key=ApplicationName,Values=cfn-teleport-test \
			--resource-type-filters s3:bucket \
			--query 'ResourceTagMappingList[].[ResourceARN]' \
			--output text | while read -r arn; do \
		echo "Deleting bucket "$$arn"..."; \
		aws s3api delete-bucket --bucket "$${arn##*:}"; \
	done; \
	aws resourcegroupstaggingapi get-resources \
			--tag-filters Key=ApplicationName,Values=cfn-teleport-test \
			--resource-type-filters dynamodb:table \
			--query 'ResourceTagMappingList[].[ResourceARN]' \
			--output text | while read -r arn; do \
		echo "Deleting table "$$arn"..."; \
		aws dynamodb delete-table --table-name "$${arn##*/}" --output text > /dev/null; \
	done; \
	aws ec2 describe-instances \
			--filters "Name=tag:ApplicationName,Values=cfn-teleport-test" \
			          "Name=instance-state-code,Values=16" \
			--query 'Reservations[].Instances[].[InstanceId]' \
			--output text | while read -r arn; do \
		echo "Deleting instance "$$arn"..."; \
		aws ec2 terminate-instances --instance-ids "$${arn##*/}" --output text > /dev/null; \
	done; \
	aws resourcegroupstaggingapi get-resources \
			--tag-filters Key=ApplicationName,Values=cfn-teleport-test \
			--resource-type-filters ec2:security-group \
			--query 'ResourceTagMappingList[].[ResourceARN]' \
			--output text | while read -r arn; do \
		echo "Deleting security-group "$$arn"..."; \
		aws ec2 delete-security-group --group-id "$${arn##*/}" --output text > /dev/null; \
	done; \
	aws ec2 delete-key-pair --key-name "cfn-teleport-test" ; \
	aws iam list-roles \
			--query 'Roles[?RoleName==`cfn-teleport-test`].[RoleName]' \
			--output text | while read -r role; do \
		aws iam list-instance-profiles \
				--query 'InstanceProfiles[?Roles[?RoleName==`cfn-teleport-test`]].[InstanceProfileName]' \
				--output text | while read -r profile; do \
			echo "Deleting instance-profile "$$profile"..."; \
			aws iam remove-role-from-instance-profile --instance-profile-name "$$profile" --role-name "$$role"; \
			aws iam delete-instance-profile --instance-profile-name "$$profile"; \
		done; \
		aws iam delete-role --role-name "$$role" ; \
		echo "Deleting role "$$role"..."; \
	done

test-reset:
	@\
	cd test/cloudformation && \
	$(MAKE) DESTROY
	@$(MAKE) test-clean-all

lint:
	@cargo fmt -- --check
	@cargo clippy -- -D warnings
