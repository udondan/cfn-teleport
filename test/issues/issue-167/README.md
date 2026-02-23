# Issue #167 Reproduction

## Error
`expected value at line 1 column 1`

## Issue Description
User experiences a JSON parsing error when attempting to migrate an S3 bucket resource between CloudFormation stacks.

## Reproduction Steps

1. **Created two CloudFormation stacks** in `eu-west-1` region:
   - **Stack1**: Contains two S3 buckets (`MyBucket1` and `SecondBucket1`)
   - **Stack2**: Contains two S3 buckets (`MyBucket2` and `SecondBucket2`)

2. **Deployed both stacks successfully** using AWS CLI

3. **Ran cfn-teleport migration command**:
   ```bash
   AWS_PROFILE=cicd:admin AWS_REGION=eu-west-1 cargo run --release -- \
     --source Stack1 \
     --target Stack2 \
     --resource MyBucket1 \
     --yes
   ```

4. **Result**: Error occurred immediately
   ```
   expected value at line 1 column 1
   ```

## Environment
- AWS Account: 031700846815
- AWS Region: eu-west-1
- cfn-teleport version: 0.47.0
- AWS Profile: cicd:admin

## Stack Details

### Stack1 Resources
- **MyBucket1**: `mybucket-031700846815-logs-1-eu-west-1`
- **SecondBucket1**: `mybucket-031700846815-logs-eu-west-1-second-bucket`

### Stack2 Resources
- **MyBucket2**: `mybucket-031700846815-logs-2-eu-west-1`
- **SecondBucket2**: `mybucket-031700846815-logs-eu-west-1-second-bucket-2`

## Analysis
The error "expected value at line 1 column 1" is a JSON parsing error, indicating that cfn-teleport is attempting to parse an empty or invalid JSON response from AWS CloudFormation API. This suggests:

1. An AWS API call is returning an empty response
2. The code is expecting JSON but receiving something else (empty string, error message, etc.)
3. Missing error handling for empty/null responses

## Next Steps
- Identify which AWS API call is returning the empty response
- Add proper error handling and logging
- Debug with AWS SDK logging enabled: `RUST_LOG=aws_config=debug,aws_sdk_cloudformation=debug`

## Cleanup
To clean up the test resources:
```bash
AWS_PROFILE=cicd:admin aws cloudformation delete-stack --stack-name Stack1 --region eu-west-1
AWS_PROFILE=cicd:admin aws cloudformation delete-stack --stack-name Stack2 --region eu-west-1
```
