#!/usr/bin/env node
import cdk = require('aws-cdk-lib');

import { TestStack } from '../lib';

const app = new cdk.App();

// Stack 1: Deployed via CDK (JSON format)
// Contains all test resources initially
new TestStack(app, 'CfnTeleportTest1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resources: true,
});

// Stack 2: Deployed via AWS CLI using YAML template (stack2-template.yaml)
// This ensures we test YAML format preservation
// Note: Stack 2 is NOT created by CDK - see Makefile deploy target
