.PHONY: phony
phony:

build:
	@npm run build

diff: build
	@cdk diff

synth: build
	@cdk synth

deploy: build
	@cdk deploy --require-approval never "*"

DESTROY:
	@cdk destroy --force "*"

info:
	@cdk doctor

clean:
	@rm -rf node_modules package-lock.json

install:
	@npm i
