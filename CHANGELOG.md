# Changelog

## [0.4.0](https://github.com/udondan/cfn-teleport/compare/v0.3.1...v0.4.0) (2023-05-11)


### Features

* Updates supported resource types (4) ([#134](https://github.com/udondan/cfn-teleport/issues/134)) ([2e466f2](https://github.com/udondan/cfn-teleport/commit/2e466f23e9af25bd4328a6727c7f2c39af72ecf6))


### Bug Fixes

* removes remains of DeletionPolicy we are required to set during import ([#126](https://github.com/udondan/cfn-teleport/issues/126)) ([02843c5](https://github.com/udondan/cfn-teleport/commit/02843c5130991dc834fcf3ab9dcf0cc09fdce798))

## [0.3.1](https://github.com/udondan/cfn-teleport/compare/v0.3.0...v0.3.1) (2023-04-30)


### Bug Fixes

* set correct default values for DeletionPolicy, when importing resources ([#111](https://github.com/udondan/cfn-teleport/issues/111)) ([c8be9a4](https://github.com/udondan/cfn-teleport/commit/c8be9a4fc0f61817517c50638149e602c3bb8588))

## [0.3.0](https://github.com/udondan/cfn-teleport/compare/v0.2.0...v0.3.0) (2023-03-30)


### Features

* show renamed resources in summary ([#66](https://github.com/udondan/cfn-teleport/issues/66)) ([09c17d5](https://github.com/udondan/cfn-teleport/commit/09c17d5c16c483454e62bc14757f693ca1393a7c))
* validating all templates before execution and optimize output ([f6b88c5](https://github.com/udondan/cfn-teleport/commit/f6b88c51b2691c8d620c7a1e1ddac0c54b89b25d))


### Bug Fixes

* adds all capabilities to the changeset, so we can process all possible resources and templates ([2f7e39f](https://github.com/udondan/cfn-teleport/commit/2f7e39f4609558160b563e344f5de886a2fae61c))
* adds DeletionPolicy to all resources as required by CFN import ([4b25a57](https://github.com/udondan/cfn-teleport/commit/4b25a57be2d97d653f59de2cc77ff295af803e5d))
* fixes invalid pipeline yaml ([416d747](https://github.com/udondan/cfn-teleport/commit/416d747c71751d67004594bfa60bd94c880aafe6))
* removes useless method call ([723dd04](https://github.com/udondan/cfn-teleport/commit/723dd04231b2d70e2ccb52a6c91bfbcb05e3f15a))
* select correct (first) resource identifier, for resources that have multiple possible identifier keys ([9e9e628](https://github.com/udondan/cfn-teleport/commit/9e9e6289099f259d7397e11c5637974cd0b8fce7))
* various fixes ([f5c5f47](https://github.com/udondan/cfn-teleport/commit/f5c5f476f657d8bb7f8b211e62dcb8540c49d09c))


### Reverts

* currently cannot delete resources, lacking permissions ([f54fdaa](https://github.com/udondan/cfn-teleport/commit/f54fdaa8317577f3f76c6e372316b88548b09ad7))
* for now, disable renaming of resources ([0f7a8db](https://github.com/udondan/cfn-teleport/commit/0f7a8dbd8843b7c2b3f2c5add51cc6f482c87dc2))

## [0.2.0](https://github.com/udondan/cfn-teleport/compare/v0.1.1...v0.2.0) (2023-03-26)


### Features

* Adds spinner when waiting for CFN actions ([#61](https://github.com/udondan/cfn-teleport/issues/61)) ([8025a7f](https://github.com/udondan/cfn-teleport/commit/8025a7ff78a0e6bd8ba72823612fe337deeaaa98))
* improves output formatting ([#65](https://github.com/udondan/cfn-teleport/issues/65)) ([c2b38f6](https://github.com/udondan/cfn-teleport/commit/c2b38f6b5cf931fe6d9fa5a92494b6d15f4ba565))


### Bug Fixes

* use pagination to get all stacks ([#59](https://github.com/udondan/cfn-teleport/issues/59)) ([6e5053b](https://github.com/udondan/cfn-teleport/commit/6e5053b5e6f27c532219b002ccf89003fbc0aeed))

## [0.1.1](https://github.com/udondan/cfn-teleport/compare/v0.1.0...v0.1.1) (2023-03-26)


### Bug Fixes

* fixes renaming of resources ([#53](https://github.com/udondan/cfn-teleport/issues/53)) ([15ec03e](https://github.com/udondan/cfn-teleport/commit/15ec03e70db615b2c95d3c11d90ed7da151f8059))

## 0.1.0 (2023-03-12)


### Features

* now usable via command line options ([#32](https://github.com/udondan/cfn-teleport/issues/32)) ([5fce44f](https://github.com/udondan/cfn-teleport/commit/5fce44fbc6d18e7affc94b2bf5635ce24d89e4fc))
