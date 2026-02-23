.PHONY: phony
phony:

build:
	@npm run build

diff: build
	@npx cdk diff

synth: build
	@npx cdk synth

deploy: build
	@echo "=========================================="
	@echo "Deploying Refactor Test Stacks"
	@echo "=========================================="
	@echo "Deploying CfnTeleportRefactorTest1 (JSON via CDK)..."
	@npx cdk deploy --require-approval never "CfnTeleportRefactorTest1"
	@echo ""
	@echo "Deploying CfnTeleportRefactorTest2 (YAML via AWS CLI)..."
	@aws cloudformation deploy \
		--template-file refactor-test2-template.yaml \
		--stack-name CfnTeleportRefactorTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "=========================================="
	@echo "Deploying Import Test Stacks"
	@echo "=========================================="
	@echo "Deploying CfnTeleportImportTest1 (JSON via CDK)..."
	@npx cdk deploy --require-approval never "CfnTeleportImportTest1"
	@echo ""
	@echo "Deploying CfnTeleportImportTest2 (YAML via AWS CLI)..."
	@aws cloudformation deploy \
		--template-file import-test2-template.yaml \
		--stack-name CfnTeleportImportTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "=========================================="
	@echo "Deploying Rename Test Stack"
	@echo "=========================================="
	@echo "Deploying CfnTeleportRenameTest1 (JSON via CDK)..."
	@npx cdk deploy --require-approval never "CfnTeleportRenameTest1"
	@echo ""
	@echo "=========================================="
	@echo "Verifying stack formats"
	@echo "=========================================="
	@$(MAKE) verify-formats

verify-formats:
	@echo "Checking JSON format stacks (CDK-deployed)..."
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
	@rm -f /tmp/template-refactor1.txt /tmp/template-refactor2.txt /tmp/template-import1.txt /tmp/template-import2.txt /tmp/template-rename1.txt
	@echo ""
	@echo "✅ All stacks have correct formats!"

DESTROY:
	@echo "Destroying Refactor Test Stacks..."
	@npx cdk destroy --force "CfnTeleportRefactorTest1" || true
	@aws cloudformation delete-stack --stack-name CfnTeleportRefactorTest2 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportRefactorTest2 || true
	@echo ""
	@echo "Destroying Import Test Stacks..."
	@npx cdk destroy --force "CfnTeleportImportTest1" || true
	@aws cloudformation delete-stack --stack-name CfnTeleportImportTest2 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportImportTest2 || true
	@echo ""
	@echo "Destroying Rename Test Stack..."
	@npx cdk destroy --force "CfnTeleportRenameTest1" || true
	@echo ""
	@echo "✅ All test stacks destroyed"

info:
	@npx cdk doctor

clean:
	@rm -rf node_modules package-lock.json

install:
	@@npm ci --cache .npm  --logs-dir .npm/$${CI_JOB_ID} --prefer-offline
