.PHONY: phony
phony:

build:
	@npm run build

diff: build
	@cdk diff

synth: build
	@cdk synth

deploy: build
	@cdk deploy --require-approval never $(notify) "*"

DESTROY:
	@cdk destroy $(notify)

info:
	@cdk doctor

clean:
	@rm -rf node_modules package-lock.json

install: clean
	@npm i
