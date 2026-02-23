import {
  aws_dynamodb,
  aws_ec2,
  aws_iam,
  aws_s3,
  aws_sqs,
  CfnOutput,
  CfnParameter,
  Fn,
  RemovalPolicy,
  Stack,
  StackProps,
  Tags,
} from 'aws-cdk-lib';
import { CfnInstanceProfile } from 'aws-cdk-lib/aws-iam';
import { CfnQueue } from 'aws-cdk-lib/aws-sqs';
import { Construct } from 'constructs';

type ResourceSet = 'refactor' | 'import' | 'rename' | 'none';

type TestStackProps = StackProps & {
  resourceSet: ResourceSet;
};

export class TestStack extends Stack {
  constructor(scope: Construct, id: string, props: TestStackProps) {
    super(scope, id, props);

    Tags.of(this).add('ApplicationName', 'cfn-teleport-test');

    // ========================================
    // PARAMETER (exists in refactor and import stacks)
    // This parameter exists in RefactorTest1/2 and ImportTest1/2 so that
    // parameter-dependent resources can be moved between stacks successfully.
    // RenameTest1 doesn't need it since rename operations are same-stack only.
    // ========================================
    const tableNameParameter = new CfnParameter(this, 'ParameterTableName', {
      type: 'String',
      default: 'cfn-teleport-param-test',
      description: 'Table name controlled by stack parameter',
    });

    // ========================================
    // REFACTOR MODE TEST RESOURCES
    // Resources for testing refactor mode cross-stack migration
    // ========================================
    if (props.resourceSet === 'refactor') {
      // Standalone bucket - no dependencies, no outputs
      new aws_s3.Bucket(this, 'StandaloneBucket', {
        bucketName: `${this.account}-cfn-teleport-standalone`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      // Standalone DynamoDB table - no dependencies, no outputs
      new aws_dynamodb.Table(this, 'StandaloneTable', {
        tableName: `cfn-teleport-standalone`,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // Regular buckets for grouped migration testing
      new aws_s3.Bucket(this, 'Bucket-1', {
        bucketName: `${this.account}-cfn-teleport-refactor-test-1`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      new aws_s3.Bucket(this, 'Bucket-2', {
        bucketName: `${this.account}-cfn-teleport-refactor-test-2`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      // DynamoDB table for grouped migration testing
      new aws_dynamodb.Table(this, 'DynamoDbTable', {
        tableName: `cfn-teleport-refactor-test`,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // Parameter-dependent table - tests parameter dependency migration
      new aws_dynamodb.Table(this, 'ParameterTable', {
        tableName: tableNameParameter.valueAsString,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // KeyPair for error testing (refactor mode should reject this)
      new aws_ec2.KeyPair(this, 'KeyPair', {
        keyPairName: 'cfn-teleport-test-refactor',
      });
    }

    // ========================================
    // IMPORT MODE TEST RESOURCES
    // Resources for testing import mode migration with KeyPair
    // ========================================
    if (props.resourceSet === 'import') {
      const vpc = aws_ec2.Vpc.fromLookup(this, 'ImportVPC', {
        isDefault: true,
      });

      // Buckets and table for import testing
      new aws_s3.Bucket(this, 'Bucket-1', {
        bucketName: `${this.account}-cfn-teleport-import-test-1`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      new aws_s3.Bucket(this, 'Bucket-2', {
        bucketName: `${this.account}-cfn-teleport-import-test-2`,
        removalPolicy: RemovalPolicy.DESTROY,
      });

      new aws_dynamodb.Table(this, 'DynamoDbTable', {
        tableName: `cfn-teleport-import-test`,
        removalPolicy: RemovalPolicy.DESTROY,
        partitionKey: {
          name: 'id',
          type: aws_dynamodb.AttributeType.STRING,
        },
      });

      // KeyPair - special case resource requiring replacement
      const keyPair = new aws_ec2.KeyPair(this, 'KeyPair', {
        keyPairName: 'cfn-teleport-test-import',
      });

      // IAM Role for Launch Template
      const role = new aws_iam.Role(this, 'Role', {
        roleName: 'cfn-teleport-import-test',
        assumedBy: new aws_iam.ServicePrincipal('ec2.amazonaws.com'),
      });

      // Instance Profile for Launch Template
      const instanceProfile = new CfnInstanceProfile(this, 'InstanceProfile', {
        instanceProfileName: 'cfn-teleport-test',
        roles: [role.roleName],
      });

      // Security Group for Launch Template
      const securityGroup = new aws_ec2.SecurityGroup(this, 'SecurityGroup', {
        securityGroupName: 'cfn-teleport-test',
        vpc,
      });

      // Machine Image lookup for Launch Template
      const machineImage = new aws_ec2.LookupMachineImage({
        name: 'amzn2-ami-hvm-*-x86_64-gp2',
        owners: ['amazon'],
      });

      // Launch Template (replaces EC2 Instance) - validates KeyPair relationships without creating instances
      new aws_ec2.CfnLaunchTemplate(this, 'LaunchTemplate', {
        launchTemplateName: 'cfn-teleport-test-template',
        launchTemplateData: {
          imageId: machineImage.getImage(this).imageId,
          instanceType: 't2.micro',
          keyName: keyPair.keyPairName,
          securityGroupIds: [securityGroup.securityGroupId],
          iamInstanceProfile: {
            arn: instanceProfile.attrArn,
          },
        },
      });
    }

    // ========================================
    // RENAME TEST RESOURCES
    // Resources for testing same-stack rename with various reference types
    // ========================================
    if (props.resourceSet === 'rename') {
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

      // 4. Resources with DependsOn relationship (for rename TEST 4)
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
    }
  }
}
