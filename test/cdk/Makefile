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
	@echo "Deploying CfnTeleportTest1 (JSON via CDK)"
	@echo "=========================================="
	@npx cdk deploy --require-approval never "CfnTeleportTest1"
	@echo ""
	@echo "=========================================="
	@echo "Deploying CfnTeleportTest2 (YAML via AWS CLI)"
	@echo "=========================================="
	@aws cloudformation deploy \
		--template-file stack2-template.yaml \
		--stack-name CfnTeleportTest2 \
		--parameter-overrides ParameterTableName=cfn-teleport-param-test \
		--tags ApplicationName=cfn-teleport-test \
		--no-fail-on-empty-changeset
	@echo ""
	@echo "=========================================="
	@echo "Verifying stack formats"
	@echo "=========================================="
	@$(MAKE) verify-formats

verify-formats:
	@echo "Checking CfnTeleportTest1 format (should be JSON)..."
	@aws cloudformation get-template --stack-name CfnTeleportTest1 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template1.txt
	@head -c 1 /tmp/template1.txt | grep -q '{' && \
		echo "✅ CfnTeleportTest1 is JSON format" || \
		(echo "❌ CfnTeleportTest1 is NOT JSON format" && exit 1)
	@echo ""
	@echo "Checking CfnTeleportTest2 format (should be YAML)..."
	@aws cloudformation get-template --stack-name CfnTeleportTest2 --query 'TemplateBody' --output json | jq -r '.' > /tmp/template2.txt
	@head -c 1 /tmp/template2.txt | grep -q '{' && \
		(echo "❌ CfnTeleportTest2 is JSON format (should be YAML)" && exit 1) || \
		echo "✅ CfnTeleportTest2 is YAML format"
	@rm -f /tmp/template1.txt /tmp/template2.txt
	@echo ""
	@echo "✅ Both stacks have correct formats!"

DESTROY:
	@echo "Destroying CfnTeleportTest1 (CDK)..."
	@npx cdk destroy --force "CfnTeleportTest1" || true
	@echo "Destroying CfnTeleportTest2 (CloudFormation)..."
	@aws cloudformation delete-stack --stack-name CfnTeleportTest2 || true
	@aws cloudformation wait stack-delete-complete --stack-name CfnTeleportTest2 || true

info:
	@npx cdk doctor

clean:
	@rm -rf node_modules package-lock.json

install:
	@@npm ci --cache .npm  --logs-dir .npm/$${CI_JOB_ID} --prefer-offline
