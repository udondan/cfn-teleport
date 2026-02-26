.PHONY: phony
phony:

deploy: phony
	@echo "=========================================="
	@echo "Deploying Refactor Test Stacks"
	@echo "=========================================="
	@echo "Deploying CfnTeleportRefactorTest1 (JSON)..."
	@aws cloudformation deploy \
		--template-file CfnTeleportRefactorTest1.json \
		--stack-name CfnTeleportRefactorTest1 \
		--capabilities CAPABILITY_NAMED_IAM \
		--parameter-overrides ParameterTableName=cfn-teleport-refactor-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "Deploying CfnTeleportRefactorTest2 (YAML)..."
	@aws cloudformation deploy \
		--template-file CfnTeleportRefactorTest2.yaml \
		--stack-name CfnTeleportRefactorTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "=========================================="
	@echo "Deploying Import Test Stacks"
	@echo "=========================================="
	@echo "Deploying CfnTeleportImportTest1 (JSON)..."
	@aws cloudformation deploy \
		--template-file CfnTeleportImportTest1.json \
		--stack-name CfnTeleportImportTest1 \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "Deploying CfnTeleportImportTest2 (YAML)..."
	@aws cloudformation deploy \
		--template-file CfnTeleportImportTest2.yaml \
		--stack-name CfnTeleportImportTest2 \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "=========================================="
	@echo "Deploying Rename Test Stack"
	@echo "=========================================="
	@echo "Deploying CfnTeleportRenameTest1 (JSON)..."
	@aws cloudformation deploy \
		--template-file CfnTeleportRenameTest1.json \
		--stack-name CfnTeleportRenameTest1 \
		--capabilities CAPABILITY_NAMED_IAM \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@$(MAKE) verify-formats

check-drift: phony
	@echo "Checking drift for stack: $(STACK) (comparing against $(TEMPLATE))"
	@aws cloudformation get-template \
		--stack-name "$(STACK)" \
		--query 'TemplateBody' \
		--output json | jq -r '.' > /tmp/deployed-$(STACK).json
	@jq -S '.Resources | keys | sort' "$(TEMPLATE)" > /tmp/original-resources.json
	@jq -S '.Resources | keys | sort' /tmp/deployed-$(STACK).json > /tmp/deployed-resources.json
	@if diff -q /tmp/original-resources.json /tmp/deployed-resources.json > /dev/null 2>&1; then \
		echo "✓ No drift detected (resources match)"; \
		if [ "$(EXPECT)" = "yes" ]; then \
			echo "✗ Expected changes but found none"; \
			echo "DEBUG: Original resources:"; \
			cat /tmp/original-resources.json; \
			echo "DEBUG: Deployed resources:"; \
			cat /tmp/deployed-resources.json; \
			exit 1; \
		fi; \
	else \
		echo "✓ Drift detected (resources differ)"; \
		echo "Resources in original template:"; \
		cat /tmp/original-resources.json; \
		echo "Resources in deployed stack:"; \
		cat /tmp/deployed-resources.json; \
		if [ "$(EXPECT)" = "no" ]; then \
			echo "✗ Expected no changes but found drift"; \
			exit 1; \
		fi; \
	fi

verify-formats: phony
	@echo "=========================================="
	@echo "Verifying stack formats"
	@echo "=========================================="
	@echo "Checking JSON format stacks (CloudFormation-deployed)..."
	@aws cloudformation get-template --stack-name CfnTeleportRefactorTest1 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template-refactor1.txt
	@head -c 1 /tmp/template-refactor1.txt | grep -q '{' && \
		echo "✅ CfnTeleportRefactorTest1 is JSON format" || \
		(echo "❌ CfnTeleportRefactorTest1 is NOT JSON format" && exit 1)
	@aws cloudformation get-template --stack-name CfnTeleportImportTest1 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template-import1.txt
	@head -c 1 /tmp/template-import1.txt | grep -q '{' && \
		echo "✅ CfnTeleportImportTest1 is JSON format" || \
		(echo "❌ CfnTeleportImportTest1 is NOT JSON format" && exit 1)
	@aws cloudformation get-template --stack-name CfnTeleportRenameTest1 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template-rename1.txt
	@head -c 1 /tmp/template-rename1.txt | grep -q '{' && \
		echo "✅ CfnTeleportRenameTest1 is JSON format" || \
		(echo "❌ CfnTeleportRenameTest1 is NOT JSON format" && exit 1)
	@echo ""
	@echo "Checking YAML format stacks (AWS CLI-deployed)..."
	@aws cloudformation get-template --stack-name CfnTeleportRefactorTest2 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template-refactor2.txt
	@head -c 1 /tmp/template-refactor2.txt | grep -q '{' && \
		(echo "❌ CfnTeleportRefactorTest2 is JSON format (should be YAML)" && exit 1) || \
		echo "✅ CfnTeleportRefactorTest2 is YAML format"
	@aws cloudformation get-template --stack-name CfnTeleportImportTest2 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template-import2.txt
	@head -c 1 /tmp/template-import2.txt | grep -q '{' && \
		(echo "❌ CfnTeleportImportTest2 is JSON format (should be YAML)" && exit 1) || \
		echo "✅ CfnTeleportImportTest2 is YAML format"
	@rm -f /tmp/template-*.txt
	@echo ""
	@echo "✅ All stacks have correct formats!"

DESTROY: phony
	@echo "Destroying Refactor Test Stacks..."
	@aws cloudformation delete-stack --stack-name CfnTeleportRefactorTest1 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportRefactorTest1 || true
	@aws cloudformation delete-stack --stack-name CfnTeleportRefactorTest2 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportRefactorTest2 || true
	@echo ""
	@echo "Destroying Import Test Stacks..."
	@aws cloudformation delete-stack --stack-name CfnTeleportImportTest1 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportImportTest1 || true
	@aws cloudformation delete-stack --stack-name CfnTeleportImportTest2 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportImportTest2 || true
	@echo ""
	@echo "Destroying Rename Test Stack..."
	@aws cloudformation delete-stack --stack-name CfnTeleportRenameTest1 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportRenameTest1 || true
	@echo ""
	@echo "✅ All test stacks destroyed"
