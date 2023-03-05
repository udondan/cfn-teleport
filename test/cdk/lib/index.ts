import { aws_s3, RemovalPolicy, Stack, StackProps } from 'aws-cdk-lib';
import { Construct } from 'constructs';

type TestStackProps = StackProps & {
  bucket: boolean;
};

export class TestStack extends Stack {
  constructor(scope: Construct, id: string, props: TestStackProps) {
    super(scope, id, props);

    if (props.bucket) {
      new aws_s3.Bucket(this, 'Bucket-1', {
        bucketName: `${this.account}-migration-test-1`,
        removalPolicy: RemovalPolicy.DESTROY,
      });
      new aws_s3.Bucket(this, 'Bucket-2', {
        bucketName: `${this.account}-migration-test-2`,
        removalPolicy: RemovalPolicy.RETAIN,
      });
    }
  }
}
