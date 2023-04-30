import {
  aws_dynamodb,
  aws_ec2,
  aws_iam,
  aws_s3,
  RemovalPolicy,
  Stack,
  StackProps,
  Tags,
} from 'aws-cdk-lib';
import { CfnInstanceProfile } from 'aws-cdk-lib/aws-iam';
import { Construct } from 'constructs';

type TestStackProps = StackProps & {
  resources: boolean;
};

export class TestStack extends Stack {
  constructor(scope: Construct, id: string, props: TestStackProps) {
    super(scope, id, props);

    Tags.of(this).add('ApplicationName', 'cfn-teleport-test');

    if (props.resources) {
      const vpc = aws_ec2.Vpc.fromLookup(this, 'ImportVPC', {
        isDefault: true,
      });

      new aws_s3.Bucket(this, 'Bucket-1', {
        bucketName: `${this.account}-cfn-teleport-test-1`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      new aws_s3.Bucket(this, 'Bucket-2', {
        bucketName: `${this.account}-cfn-teleport-test-2`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      new aws_dynamodb.Table(this, 'DynamoDbTable', {
        tableName: `cfn-teleport-test`,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      const keyPair = new aws_ec2.CfnKeyPair(this, 'KeyPair', {
        keyName: 'cfn-teleport-test',
      });

      const role = new aws_iam.Role(this, 'Role', {
        roleName: 'cfn-teleport-test',
        assumedBy: new aws_iam.ServicePrincipal('ec2.amazonaws.com'),
      });

      const securityGroup = new aws_ec2.SecurityGroup(this, 'SecurityGroup', {
        securityGroupName: 'cfn-teleport-test',
        vpc,
      });

      const machineImage = new aws_ec2.LookupMachineImage({
        name: 'amzn2-ami-hvm-*-x86_64-gp2',
        owners: ['amazon'],
      });

      CfnInstanceProfile;

      new aws_ec2.Instance(this, 'Instance', {
        vpc,
        machineImage,
        securityGroup,
        role,
        instanceType: aws_ec2.InstanceType.of(
          aws_ec2.InstanceClass.T2,
          aws_ec2.InstanceSize.MICRO
        ),
        keyName: keyPair.keyName,
      });
    }
  }
}
