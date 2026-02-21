import {
  aws_dynamodb,
  aws_ec2,
  aws_iam,
  aws_s3,
  aws_sqs,
  CfnOutput,
  Fn,
  RemovalPolicy,
  Stack,
  StackProps,
  Tags,
} from 'aws-cdk-lib';
import { CfnInstanceProfile } from 'aws-cdk-lib/aws-iam';
import { CfnQueue } from 'aws-cdk-lib/aws-sqs';
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
          aws_ec2.InstanceSize.MICRO,
        ),
        keyName: keyPair.keyName,
      });

      // ========================================
      // RENAME TEST RESOURCES
      // Comprehensive test resources covering all reference types
      // ========================================

      // 1. Bucket for rename testing - will be referenced by Output (Ref)
      const renameBucket = new aws_s3.Bucket(this, 'RenameBucket', {
        bucketName: `${this.account}-cfn-teleport-rename-test`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      // Output that references RenameBucket (uses Ref)
      new CfnOutput(this, 'RenameBucketOutput', {
        value: renameBucket.bucketName,
        description: 'Bucket name for rename testing (Ref)',
      });

      // 2. Table for rename testing - will be referenced by Output (GetAtt)
      const renameTable = new aws_dynamodb.Table(this, 'RenameTable', {
        tableName: `cfn-teleport-rename-test`,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // Output that references RenameTable (uses GetAtt)
      new CfnOutput(this, 'RenameTableOutput', {
        value: renameTable.tableArn,
        description: 'Table ARN for rename testing (GetAtt)',
      });

      // 3. Queue for Fn::Sub testing - will be referenced in Output
      const renameQueue = new aws_sqs.Queue(this, 'RenameQueue', {
        queueName: 'cfn-teleport-rename-test-queue',
        removalPolicy: RemovalPolicy.DESTROY,
      });

      // Get the L1 CFN resource to access logical ID
      const cfnQueue = renameQueue.node.defaultChild as CfnQueue;

      // Output using Fn::Sub with both pseudo-params and resource reference
      new CfnOutput(this, 'SubTestOutput', {
        value: Fn.sub('Queue ${QueueRef} in region ${AWS::Region}', {
          QueueRef: cfnQueue.ref, // This creates a Ref to the queue
        }),
        description: 'Output using Fn::Sub with resource reference',
      });

      // 5. Resources with DependsOn relationship
      const dependencyBucket = new aws_s3.Bucket(this, 'DependencyBucket', {
        bucketName: `${this.account}-cfn-teleport-dependency-test`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      const dependentTable = new aws_dynamodb.Table(this, 'DependentTable', {
        tableName: 'cfn-teleport-dependent-table',
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // Add DependsOn at CFN level
      const cfnDependentTable = dependentTable.node
        .defaultChild as aws_dynamodb.CfnTable;
      cfnDependentTable.addDependency(
        dependencyBucket.node.defaultChild as aws_s3.CfnBucket,
      );

      // Output to verify DependsOn resources exist
      new CfnOutput(this, 'DependencyTestOutput', {
        value: `${dependencyBucket.bucketName}:${dependentTable.tableName}`,
        description: 'Resources with DependsOn relationship',
      });

      // 6. Bucket policy that references another bucket (resource property reference)
      const policyTargetBucket = new aws_s3.Bucket(this, 'PolicyTargetBucket', {
        bucketName: `${this.account}-cfn-teleport-policy-target`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      const policySourceBucket = new aws_s3.Bucket(this, 'PolicySourceBucket', {
        bucketName: `${this.account}-cfn-teleport-policy-source`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      // Add bucket policy that references PolicyTargetBucket
      policySourceBucket.addToResourcePolicy(
        new aws_iam.PolicyStatement({
          effect: aws_iam.Effect.ALLOW,
          principals: [new aws_iam.AccountRootPrincipal()],
          actions: ['s3:GetObject'],
          resources: [`${policyTargetBucket.bucketArn}/*`], // References PolicyTargetBucket via GetAtt
        }),
      );

      new CfnOutput(this, 'PolicyTestOutput', {
        value: `${policySourceBucket.bucketName}:${policyTargetBucket.bucketName}`,
        description: 'Buckets with policy cross-reference',
      });
    }
  }
}
