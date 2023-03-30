#!/usr/bin/env node
import cdk = require('aws-cdk-lib');

import { TestStack } from '../lib';

const app = new cdk.App();
new TestStack(app, 'CfnTeleportTest1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resources: true,
});

new TestStack(app, 'CfnTeleportTest2', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  resources: false,
});
