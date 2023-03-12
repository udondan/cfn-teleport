.PHONY: phony
phony:

build:
	@npm run build

diff: build
	@npx cdk diff

synth: build
	@npx cdk synth

deploy: build
	@npx cdk deploy --require-approval never "*"

DESTROY:
	@npx cdk destroy --force "*"

info:
	@npx cdk doctor

clean:
	@rm -rf node_modules package-lock.json

install:
	@npm ci
