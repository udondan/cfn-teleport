# Changelog

## [0.48.0](https://github.com/udondan/cfn-teleport/compare/v0.47.0...v0.48.0) (2026-03-01)


### Features

* add retry logic and detailed error messages for CloudFormation API calls ([#1304](https://github.com/udondan/cfn-teleport/issues/1304)) ([62c40c2](https://github.com/udondan/cfn-teleport/commit/62c40c21b827b35dc6c108c975cc50877c80d85b))
* add short option -m for --mode flag ([#1301](https://github.com/udondan/cfn-teleport/issues/1301)) ([7f5a741](https://github.com/udondan/cfn-teleport/commit/7f5a74186405eafbdee6a4550515b96b2cb7e2f0))
* add support for YAML CloudFormation templates ([#1267](https://github.com/udondan/cfn-teleport/issues/1267)) ([1536f34](https://github.com/udondan/cfn-teleport/commit/1536f34b13d2df16eb320c20ee2f072e67970da0))
* add template I/O support for export and file-based migrations ([#1302](https://github.com/udondan/cfn-teleport/issues/1302)) ([a97f0fe](https://github.com/udondan/cfn-teleport/commit/a97f0fe29260121e48ba6a06c879ab3ee7f90b09))
* add Windows executable metadata ([#1297](https://github.com/udondan/cfn-teleport/issues/1297)) ([3c06fa6](https://github.com/udondan/cfn-teleport/commit/3c06fa6a078f223f34f8034110b47c9cbdc265b6))
* improve dependency validation error messages ([#1265](https://github.com/udondan/cfn-teleport/issues/1265)) ([d006a80](https://github.com/udondan/cfn-teleport/commit/d006a8016239b9de657d444e5a815bc5f5f3be98))
* migrate Linux builds to musl for better compatibility ([#1308](https://github.com/udondan/cfn-teleport/issues/1308)) ([d1ef351](https://github.com/udondan/cfn-teleport/commit/d1ef3516cb1d322b0f7a136d765b099fbd4cce85))


### Bug Fixes

* change MultiSelect unchecked item color from purple to grey ([#1287](https://github.com/udondan/cfn-teleport/issues/1287)) ([e1cb9cb](https://github.com/udondan/cfn-teleport/commit/e1cb9cb0f41d286e7e4c4eccc09a4075b9ce1560))

## [0.47.0](https://github.com/udondan/cfn-teleport/compare/v0.46.0...v0.47.0) (2026-02-22)


### Features

* add --mode parameter to choose between refactor and import methods ([#1259](https://github.com/udondan/cfn-teleport/issues/1259)) ([83180b3](https://github.com/udondan/cfn-teleport/commit/83180b3e3926fd28211c537816f566a5998e0313))
* add comprehensive same-stack resource renaming with automatic reference updates ([#1257](https://github.com/udondan/cfn-teleport/issues/1257)) ([cfdf8d9](https://github.com/udondan/cfn-teleport/commit/cfdf8d9ba5768d083309d94c820893131a63641d))
* add visual dependency markers to resource selection UI ([#1261](https://github.com/udondan/cfn-teleport/issues/1261)) ([7d2056a](https://github.com/udondan/cfn-teleport/commit/7d2056ac8e81351b9cdad421caef756838f43858))


### Bug Fixes

* centralize error handling to properly display error messages with newlines ([#1264](https://github.com/udondan/cfn-teleport/issues/1264)) ([2b8994d](https://github.com/udondan/cfn-teleport/commit/2b8994d64a07f2b04cf46a8e5a08bc02d9d478b4))

## [0.46.0](https://github.com/udondan/cfn-teleport/compare/v0.45.0...v0.46.0) (2026-02-08)


### Features

* Updates supported resource types (43) ([#1173](https://github.com/udondan/cfn-teleport/issues/1173)) ([77fbd5d](https://github.com/udondan/cfn-teleport/commit/77fbd5d0e2e866ade03ab389eb057d39b50e1c87))

## [0.45.0](https://github.com/udondan/cfn-teleport/compare/v0.44.0...v0.45.0) (2025-11-13)


### Features

* Updates supported resource types (5) ([#1164](https://github.com/udondan/cfn-teleport/issues/1164)) ([bc8b653](https://github.com/udondan/cfn-teleport/commit/bc8b65372cdbec59c5ddc359fdfefb26aba7a40b))

## [0.44.0](https://github.com/udondan/cfn-teleport/compare/v0.43.0...v0.44.0) (2025-11-01)


### Features

* Updates supported resource types (4) ([#1156](https://github.com/udondan/cfn-teleport/issues/1156)) ([554192e](https://github.com/udondan/cfn-teleport/commit/554192e5dc4f1e4d99cb01603f57b8dfff38004f))


### Bug Fixes

* display user-friendly error when AWS credentials are missing ([#1149](https://github.com/udondan/cfn-teleport/issues/1149)) ([7ec3f22](https://github.com/udondan/cfn-teleport/commit/7ec3f22901b8dc9973ce8d9a28dfecaf2c658dcd))

## [0.43.0](https://github.com/udondan/cfn-teleport/compare/v0.42.0...v0.43.0) (2025-10-25)


### Features

* Updates supported resource types (3) ([#1145](https://github.com/udondan/cfn-teleport/issues/1145)) ([54d77b7](https://github.com/udondan/cfn-teleport/commit/54d77b711509a3681fb7fb096e448023ff9660b8))

## [0.42.0](https://github.com/udondan/cfn-teleport/compare/v0.41.0...v0.42.0) (2025-10-19)


### Features

* Updates supported resource types (4) ([#1134](https://github.com/udondan/cfn-teleport/issues/1134)) ([60e82bd](https://github.com/udondan/cfn-teleport/commit/60e82bdf55979bcefa3bcc2d6b98ad643d99ae12))

## [0.41.0](https://github.com/udondan/cfn-teleport/compare/v0.40.0...v0.41.0) (2025-10-11)


### Features

* Updates supported resource types (6) ([#1126](https://github.com/udondan/cfn-teleport/issues/1126)) ([c6847bb](https://github.com/udondan/cfn-teleport/commit/c6847bb159d5617767dbcbb898cc5d68aff3696a))

## [0.40.0](https://github.com/udondan/cfn-teleport/compare/v0.39.0...v0.40.0) (2025-10-04)


### Features

* Updates supported resource types (2) ([#1122](https://github.com/udondan/cfn-teleport/issues/1122)) ([99f1b24](https://github.com/udondan/cfn-teleport/commit/99f1b243e64f6ff976f10595ef67bfd86ee4f7d5))
* Updates supported resource types (6) ([#1117](https://github.com/udondan/cfn-teleport/issues/1117)) ([d49e630](https://github.com/udondan/cfn-teleport/commit/d49e630ab8689e56871231a0b5f44e72c0a932f5))
* Updates supported resource types (7) ([#1121](https://github.com/udondan/cfn-teleport/issues/1121)) ([ec3e2d3](https://github.com/udondan/cfn-teleport/commit/ec3e2d3fb54caaf206c6e0840572e6a048541112))

## [0.39.0](https://github.com/udondan/cfn-teleport/compare/v0.38.0...v0.39.0) (2025-09-26)


### Features

* Updates supported resource types (6) ([#1110](https://github.com/udondan/cfn-teleport/issues/1110)) ([484e5e5](https://github.com/udondan/cfn-teleport/commit/484e5e5d2397b9f0e80e712165b2d7d512b3515b))

## [0.38.0](https://github.com/udondan/cfn-teleport/compare/v0.37.2...v0.38.0) (2025-09-20)


### Features

* Updates supported resource types (3) ([#1105](https://github.com/udondan/cfn-teleport/issues/1105)) ([07d0e10](https://github.com/udondan/cfn-teleport/commit/07d0e108557399b82cf176a5394b8aab24391453))

## [0.37.2](https://github.com/udondan/cfn-teleport/compare/v0.37.1...v0.37.2) (2025-09-14)


### Bug Fixes

* update spinner.rs for spinach v3 API compatibility ([#1096](https://github.com/udondan/cfn-teleport/issues/1096)) ([869ef1c](https://github.com/udondan/cfn-teleport/commit/869ef1c17c1130f50fe84fb408b0211baeb38c92))

## [0.37.1](https://github.com/udondan/cfn-teleport/compare/v0.37.0...v0.37.1) (2025-09-14)


### Bug Fixes

* update artifact actions from v3 to v4/v5 ([#1094](https://github.com/udondan/cfn-teleport/issues/1094)) ([c01da26](https://github.com/udondan/cfn-teleport/commit/c01da26ce6d348fb78a1e0cd574be0c8738b1699))

## [0.37.0](https://github.com/udondan/cfn-teleport/compare/v0.36.0...v0.37.0) (2025-09-14)


### Features

* Updates supported resource types (148) ([#857](https://github.com/udondan/cfn-teleport/issues/857)) ([668e529](https://github.com/udondan/cfn-teleport/commit/668e5299e6cd236b938fb92301d886d474f70037))


### Bug Fixes

* converts large array to static & updates BehaviorVersion of aws_config to latest (v2025_08_07) ([#1092](https://github.com/udondan/cfn-teleport/issues/1092)) ([3955abc](https://github.com/udondan/cfn-teleport/commit/3955abcfcd992445989772cf8e24788ffaf83473))

## [0.36.0](https://github.com/udondan/cfn-teleport/compare/v0.35.0...v0.36.0) (2025-01-04)


### Features

* Updates supported resource types (1) ([#841](https://github.com/udondan/cfn-teleport/issues/841)) ([3b69d1d](https://github.com/udondan/cfn-teleport/commit/3b69d1df029bbb463f41200bc6aaa454de0ed59c))
* Updates supported resource types (1) ([#845](https://github.com/udondan/cfn-teleport/issues/845)) ([8d74695](https://github.com/udondan/cfn-teleport/commit/8d7469545f9037f45b23384eecb4b5d90b605ff8))

## [0.35.0](https://github.com/udondan/cfn-teleport/compare/v0.34.0...v0.35.0) (2024-12-21)


### Features

* Updates supported resource types (6) ([#835](https://github.com/udondan/cfn-teleport/issues/835)) ([fd7888d](https://github.com/udondan/cfn-teleport/commit/fd7888db8fb0d8998612f37c393be3b34fad3144))

## [0.34.0](https://github.com/udondan/cfn-teleport/compare/v0.33.0...v0.34.0) (2024-12-14)


### Features

* Updates supported resource types (7) ([#829](https://github.com/udondan/cfn-teleport/issues/829)) ([c24e7ff](https://github.com/udondan/cfn-teleport/commit/c24e7ff1a89705a3f0c3289aa05ecc02bfbb0adc))

## [0.33.0](https://github.com/udondan/cfn-teleport/compare/v0.32.0...v0.33.0) (2024-12-07)


### Features

* Updates supported resource types (7) ([#825](https://github.com/udondan/cfn-teleport/issues/825)) ([ab4eeec](https://github.com/udondan/cfn-teleport/commit/ab4eeec9f8d429131a255218280ae7da28eba570))

## [0.32.0](https://github.com/udondan/cfn-teleport/compare/v0.31.0...v0.32.0) (2024-11-23)


### Features

* Updates supported resource types (14) ([#814](https://github.com/udondan/cfn-teleport/issues/814)) ([db6aecc](https://github.com/udondan/cfn-teleport/commit/db6aecca55c618982a6ae5ef0be03e93530c6ce8))
* Updates supported resource types (2) ([#805](https://github.com/udondan/cfn-teleport/issues/805)) ([4a7a700](https://github.com/udondan/cfn-teleport/commit/4a7a70083d6fd7c29b0c8808773f9f28bd2dc896))

## [0.31.0](https://github.com/udondan/cfn-teleport/compare/v0.30.0...v0.31.0) (2024-11-17)


### Features

* Updates supported resource types (1) ([#800](https://github.com/udondan/cfn-teleport/issues/800)) ([e264150](https://github.com/udondan/cfn-teleport/commit/e264150d08d02dcf6420da038556c9864108f19e))

## [0.30.0](https://github.com/udondan/cfn-teleport/compare/v0.29.0...v0.30.0) (2024-11-09)


### Features

* Updates supported resource types (14) ([#793](https://github.com/udondan/cfn-teleport/issues/793)) ([7635278](https://github.com/udondan/cfn-teleport/commit/7635278b5474946b60b372d1ac626190482e1855))
* Updates supported resource types (3) ([#796](https://github.com/udondan/cfn-teleport/issues/796)) ([e241bf4](https://github.com/udondan/cfn-teleport/commit/e241bf48c51ab077cf8178a4fb42b774035b3a2c))

## [0.29.0](https://github.com/udondan/cfn-teleport/compare/v0.28.0...v0.29.0) (2024-10-25)


### Features

* Updates supported resource types (1) ([#776](https://github.com/udondan/cfn-teleport/issues/776)) ([c51eb3c](https://github.com/udondan/cfn-teleport/commit/c51eb3c60e1ac3938d421dab55e913319a918085))

## [0.28.0](https://github.com/udondan/cfn-teleport/compare/v0.27.0...v0.28.0) (2024-10-19)


### Features

* Updates supported resource types (6) ([#766](https://github.com/udondan/cfn-teleport/issues/766)) ([96a5fe5](https://github.com/udondan/cfn-teleport/commit/96a5fe5ddbe9e338fc4fc15274ec730495d39df6))

## [0.27.0](https://github.com/udondan/cfn-teleport/compare/v0.26.0...v0.27.0) (2024-10-05)


### Features

* Updates supported resource types (3) ([#742](https://github.com/udondan/cfn-teleport/issues/742)) ([0f13b59](https://github.com/udondan/cfn-teleport/commit/0f13b5906e6d735dc69fc295842a7f7f95b0332e))

## [0.26.0](https://github.com/udondan/cfn-teleport/compare/v0.25.0...v0.26.0) (2024-09-28)


### Features

* Updates supported resource types (4) ([#738](https://github.com/udondan/cfn-teleport/issues/738)) ([5700d0c](https://github.com/udondan/cfn-teleport/commit/5700d0c544bcdb61324ef0a73c0e0cb307fd88fe))

## [0.25.0](https://github.com/udondan/cfn-teleport/compare/v0.24.0...v0.25.0) (2024-09-22)


### Features

* Updates supported resource types (55) ([#619](https://github.com/udondan/cfn-teleport/issues/619)) ([0008239](https://github.com/udondan/cfn-teleport/commit/0008239a19d68fb96cce8281ce1c345fc130191d))

## [0.24.0](https://github.com/udondan/cfn-teleport/compare/v0.23.0...v0.24.0) (2024-05-17)


### Features

* Updates supported resource types (6) ([#607](https://github.com/udondan/cfn-teleport/issues/607)) ([f512e72](https://github.com/udondan/cfn-teleport/commit/f512e721e3f69f631649df2870c88417a50419d4))

## [0.23.0](https://github.com/udondan/cfn-teleport/compare/v0.22.0...v0.23.0) (2024-05-03)


### Features

* Updates supported resource types (7) ([#596](https://github.com/udondan/cfn-teleport/issues/596)) ([e51412a](https://github.com/udondan/cfn-teleport/commit/e51412a00b557dbbc58ba7b79a1f0934a4bd4263))

## [0.22.0](https://github.com/udondan/cfn-teleport/compare/v0.21.0...v0.22.0) (2024-04-27)


### Features

* Updates supported resource types (3) ([#587](https://github.com/udondan/cfn-teleport/issues/587)) ([c389463](https://github.com/udondan/cfn-teleport/commit/c389463bff3530de19db633fc55145c979943c06))
* Updates supported resource types (6) ([#590](https://github.com/udondan/cfn-teleport/issues/590)) ([a252a67](https://github.com/udondan/cfn-teleport/commit/a252a67d9af5abdd7b408e05e1b87b467956b687))

## [0.21.0](https://github.com/udondan/cfn-teleport/compare/v0.20.0...v0.21.0) (2024-04-13)


### Features

* Updates supported resource types (2) ([#579](https://github.com/udondan/cfn-teleport/issues/579)) ([1778d46](https://github.com/udondan/cfn-teleport/commit/1778d46b1ac906d06d4c101429c546cc5132a779))

## [0.20.0](https://github.com/udondan/cfn-teleport/compare/v0.19.0...v0.20.0) (2024-04-06)


### Features

* Updates supported resource types (23) ([#565](https://github.com/udondan/cfn-teleport/issues/565)) ([e1f067f](https://github.com/udondan/cfn-teleport/commit/e1f067f459fe9859a0d863214bbab522738241d0))

## [0.19.0](https://github.com/udondan/cfn-teleport/compare/v0.18.0...v0.19.0) (2024-03-30)


### Features

* Updates supported resource types (5) ([#557](https://github.com/udondan/cfn-teleport/issues/557)) ([05a1bc2](https://github.com/udondan/cfn-teleport/commit/05a1bc2c088c48a55d04f244d8f345d56ba72f88))

## [0.18.0](https://github.com/udondan/cfn-teleport/compare/v0.17.0...v0.18.0) (2024-03-24)


### Features

* Updates supported resource types (5) ([#550](https://github.com/udondan/cfn-teleport/issues/550)) ([3cd40d1](https://github.com/udondan/cfn-teleport/commit/3cd40d1386d285abf4c909e50543ee7303e1f812))
* Updates supported resource types (5) ([#551](https://github.com/udondan/cfn-teleport/issues/551)) ([b75c9d3](https://github.com/udondan/cfn-teleport/commit/b75c9d33b5363a7d50816147ff778f0a6fb69ead))


### Bug Fixes

* ensure we do not index duplicate resource types ([#548](https://github.com/udondan/cfn-teleport/issues/548)) ([bb6e3ba](https://github.com/udondan/cfn-teleport/commit/bb6e3ba6d53d8de433b6d5ad088d704e6eb453a6))

## [0.17.0](https://github.com/udondan/cfn-teleport/compare/v0.16.0...v0.17.0) (2024-03-09)


### Features

* Updates supported resource types (1) ([#528](https://github.com/udondan/cfn-teleport/issues/528)) ([b4fa4eb](https://github.com/udondan/cfn-teleport/commit/b4fa4eba472cdb9aea3a5a028e6d0776d4cac5ff))

## [0.16.0](https://github.com/udondan/cfn-teleport/compare/v0.15.0...v0.16.0) (2024-03-02)


### Features

* Updates supported resource types (3) ([#513](https://github.com/udondan/cfn-teleport/issues/513)) ([3e69db8](https://github.com/udondan/cfn-teleport/commit/3e69db80592d7668ea5eba2d658a245f79a76ec1))

## [0.15.0](https://github.com/udondan/cfn-teleport/compare/v0.14.0...v0.15.0) (2024-02-23)


### Features

* Updates supported resource types (1) ([#509](https://github.com/udondan/cfn-teleport/issues/509)) ([cfe9ff1](https://github.com/udondan/cfn-teleport/commit/cfe9ff12661c4658758d4c49b05674f94be1f4f1))

## [0.14.0](https://github.com/udondan/cfn-teleport/compare/v0.13.0...v0.14.0) (2024-02-18)


### Features

* Updates supported resource types (6) ([#499](https://github.com/udondan/cfn-teleport/issues/499)) ([4fc7adf](https://github.com/udondan/cfn-teleport/commit/4fc7adf05da3e3e2c5c25ff982895cef1e0b3104))

## [0.13.0](https://github.com/udondan/cfn-teleport/compare/v0.12.0...v0.13.0) (2024-02-10)


### Features

* Updates supported resource types (4) ([#485](https://github.com/udondan/cfn-teleport/issues/485)) ([7123002](https://github.com/udondan/cfn-teleport/commit/7123002e6b009ab2b909c77c46e0ab409e0d3f35))

## [0.12.0](https://github.com/udondan/cfn-teleport/compare/v0.11.0...v0.12.0) (2024-01-21)


### Features

* Updates supported resource types (17) ([#436](https://github.com/udondan/cfn-teleport/issues/436)) ([3a29216](https://github.com/udondan/cfn-teleport/commit/3a29216a5789bf7170543b61141f99c61af7f9ef))

## [0.11.0](https://github.com/udondan/cfn-teleport/compare/v0.10.0...v0.11.0) (2023-12-30)


### Features

* Updates supported resource types (326) ([#323](https://github.com/udondan/cfn-teleport/issues/323)) ([ecaec1e](https://github.com/udondan/cfn-teleport/commit/ecaec1efe461cbb31e989b046b29bece61d4e539))

## [0.10.0](https://github.com/udondan/cfn-teleport/compare/v0.9.0...v0.10.0) (2023-09-09)


### Features

* Updates supported resource types (5) ([#263](https://github.com/udondan/cfn-teleport/issues/263)) ([1524d9c](https://github.com/udondan/cfn-teleport/commit/1524d9c85d625fc6ce2102c2907c700be28f21a4))

## [0.9.0](https://github.com/udondan/cfn-teleport/compare/v0.8.0...v0.9.0) (2023-08-10)


### Features

* Updates supported resource types (1) ([#242](https://github.com/udondan/cfn-teleport/issues/242)) ([7f0b6f2](https://github.com/udondan/cfn-teleport/commit/7f0b6f20961fd96113a791bf236cbbfb3292d842))
* Updates supported resource types (1) ([#250](https://github.com/udondan/cfn-teleport/issues/250)) ([9cb60ba](https://github.com/udondan/cfn-teleport/commit/9cb60ba283b6d27f4179e68ac12589fb34e2a172))

## [0.8.0](https://github.com/udondan/cfn-teleport/compare/v0.7.0...v0.8.0) (2023-07-21)


### Features

* Updates supported resource types (2) ([#225](https://github.com/udondan/cfn-teleport/issues/225)) ([484b842](https://github.com/udondan/cfn-teleport/commit/484b84263bbdd0df059a5b8424022c2810367bcb))

## [0.7.0](https://github.com/udondan/cfn-teleport/compare/v0.6.0...v0.7.0) (2023-07-10)


### Features

* Updates supported resource types (1) ([#218](https://github.com/udondan/cfn-teleport/issues/218)) ([b98943e](https://github.com/udondan/cfn-teleport/commit/b98943ed5e4da68cf6d1346cb5c87340cf033117))

## [0.6.0](https://github.com/udondan/cfn-teleport/compare/v0.5.0...v0.6.0) (2023-07-05)


### Features

* Updates supported resource types (1) ([#210](https://github.com/udondan/cfn-teleport/issues/210)) ([af05043](https://github.com/udondan/cfn-teleport/commit/af05043380d7ddd7fcda34cd01c6a0eaf412ef49))

## [0.5.0](https://github.com/udondan/cfn-teleport/compare/v0.4.0...v0.5.0) (2023-06-26)


### Features

* Updates supported resource types (1) ([#190](https://github.com/udondan/cfn-teleport/issues/190)) ([bb662a5](https://github.com/udondan/cfn-teleport/commit/bb662a5a1cf1b0da3d8fad9cd5b523ffde1a4e86))

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
