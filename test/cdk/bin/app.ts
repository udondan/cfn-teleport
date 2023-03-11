#!/usr/bin/env node
import cdk = require('aws-cdk-lib');

import { TestStack } from '../lib';

const app = new cdk.App();
new TestStack(app, 'A1', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  bucket: true,
});

new TestStack(app, 'A2', {
  env: {
    account: process.env.CDK_DEFAULT_ACCOUNT,
    region: process.env.CDK_DEFAULT_REGION,
  },
  bucket: false,
});
