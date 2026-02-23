#!/usr/bin/env node
import cdk = require('aws-cdk-lib');

import { TestStack } from '../lib';

const app = new cdk.App();

// Refactor Test Stack (JSON format via CDK)
// Contains resources for testing refactor mode cross-stack migration
new TestStack(app, 'CfnTeleportRefactorTest1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resourceSet: 'refactor',
});

// Import Test Stack (JSON format via CDK)
// Contains resources for testing import mode with KeyPair and Launch Template
new TestStack(app, 'CfnTeleportImportTest1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resourceSet: 'import',
});

// Rename Test Stack (JSON format via CDK)
// Contains resources for testing same-stack rename operations
new TestStack(app, 'CfnTeleportRenameTest1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resourceSet: 'rename',
});

// Target stacks (RefactorTest2, ImportTest2) are deployed via AWS CLI
// using YAML templates to ensure YAML format preservation testing
// See Makefile deploy target for YAML stack deployment
