import { aws_s3, Stack, StackProps } from 'aws-cdk-lib';
import { Construct } from 'constructs';

type TestStackProps = StackProps & {
  bucket: boolean;
};

export class TestStack extends Stack {
  constructor(scope: Construct, id: string, props: TestStackProps) {
    super(scope, id, props);

    if (props.bucket) {
      new aws_s3.Bucket(this, 'Bucket', {
        bucketName: `${this.account}-migration-test`,
      });
    }
  }
}
