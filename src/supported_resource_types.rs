pub const SUPPORTED_RESOURCE_TYPES: [&str; 941] = [
    "AWS::ACMPCA::Certificate",
    "AWS::ACMPCA::CertificateAuthority",
    "AWS::ACMPCA::CertificateAuthorityActivation",
    "AWS::ACMPCA::Permission",
    "AWS::APS::RuleGroupsNamespace",
    "AWS::APS::Workspace",
    "AWS::AccessAnalyzer::Analyzer",
    "AWS::Amplify::App",
    "AWS::Amplify::Branch",
    "AWS::Amplify::Domain",
    "AWS::AmplifyUIBuilder::Component",
    "AWS::AmplifyUIBuilder::Form",
    "AWS::AmplifyUIBuilder::Theme",
    "AWS::ApiGateway::Account",
    "AWS::ApiGateway::ApiKey",
    "AWS::ApiGateway::Authorizer",
    "AWS::ApiGateway::BasePathMapping",
    "AWS::ApiGateway::ClientCertificate",
    "AWS::ApiGateway::Deployment",
    "AWS::ApiGateway::DocumentationPart",
    "AWS::ApiGateway::DocumentationVersion",
    "AWS::ApiGateway::DomainName",
    "AWS::ApiGateway::Method",
    "AWS::ApiGateway::Model",
    "AWS::ApiGateway::RequestValidator",
    "AWS::ApiGateway::Resource",
    "AWS::ApiGateway::RestApi",
    "AWS::ApiGateway::Stage",
    "AWS::ApiGateway::UsagePlan",
    "AWS::ApiGateway::UsagePlanKey",
    "AWS::ApiGateway::VpcLink",
    "AWS::ApiGatewayV2::Api",
    "AWS::ApiGatewayV2::ApiMapping",
    "AWS::ApiGatewayV2::Authorizer",
    "AWS::ApiGatewayV2::Deployment",
    "AWS::ApiGatewayV2::DomainName",
    "AWS::ApiGatewayV2::IntegrationResponse",
    "AWS::ApiGatewayV2::Model",
    "AWS::ApiGatewayV2::Route",
    "AWS::ApiGatewayV2::RouteResponse",
    "AWS::ApiGatewayV2::VpcLink",
    "AWS::AppConfig::ConfigurationProfile",
    "AWS::AppConfig::Environment",
    "AWS::AppConfig::Extension",
    "AWS::AppConfig::ExtensionAssociation",
    "AWS::AppConfig::HostedConfigurationVersion",
    "AWS::AppFlow::Connector",
    "AWS::AppFlow::ConnectorProfile",
    "AWS::AppFlow::Flow",
    "AWS::AppIntegrations::Application",
    "AWS::AppIntegrations::DataIntegration",
    "AWS::AppIntegrations::EventIntegration",
    "AWS::CodeArtifact::PackageGroup",
    "AWS::ApplicationAutoScaling::ScalableTarget",
    "AWS::ApplicationAutoScaling::ScalingPolicy",
    "AWS::AppRunner::AutoScalingConfiguration",
    "AWS::AppRunner::ObservabilityConfiguration",
    "AWS::AppRunner::Service",
    "AWS::AppRunner::VpcIngressConnection",
    "AWS::AppRunner::VpcConnector",
    "AWS::AppStream::AppBlock",
    "AWS::AppStream::AppBlockBuilder",
    "AWS::AppStream::Application",
    "AWS::AppStream::ApplicationEntitlementAssociation",
    "AWS::AppStream::ApplicationFleetAssociation",
    "AWS::AppStream::DirectoryConfig",
    "AWS::AppStream::Entitlement",
    "AWS::AppStream::ImageBuilder",
    "AWS::AppSync::DomainName",
    "AWS::AppSync::DomainNameApiAssociation",
    "AWS::AppSync::FunctionConfiguration",
    "AWS::AppSync::Resolver",
    "AWS::AppSync::SourceApiAssociation",
    "AWS::ApplicationInsights::Application",
    "AWS::ARCZonalShift::ZonalAutoshiftConfiguration",
    "AWS::Athena::CapacityReservation",
    "AWS::Athena::CapacityReservation",
    "AWS::Athena::DataCatalog",
    "AWS::Athena::NamedQuery",
    "AWS::Athena::PreparedStatement",
    "AWS::Athena::WorkGroup",
    "AWS::AuditManager::Assessment",
    "AWS::AutoScaling::AutoScalingGroup",
    "AWS::AutoScaling::LaunchConfiguration",
    "AWS::AutoScaling::LifecycleHook",
    "AWS::AutoScaling::ScalingPolicy",
    "AWS::AutoScaling::ScheduledAction",
    "AWS::AutoScaling::WarmPool",
    "AWS::B2BI::Capability",
    "AWS::B2BI::Partnership",
    "AWS::B2BI::Profile",
    "AWS::B2BI::Transformer",
    "AWS::Backup::BackupPlan",
    "AWS::Backup::BackupSelection",
    "AWS::Backup::BackupVault",
    "AWS::Backup::Framework",
    "AWS::Backup::ReportPlan",
    "AWS::Backup::RestoreTestingPlan",
    "AWS::Backup::RestoreTestingPlan",
    "AWS::BackupGateway::Hypervisor",
    "AWS::Batch::ComputeEnvironment",
    "AWS::Batch::JobQueue",
    "AWS::Batch::SchedulingPolicy",
    "AWS::BillingConductor::BillingGroup",
    "AWS::BillingConductor::CustomLineItem",
    "AWS::BillingConductor::PricingPlan",
    "AWS::BillingConductor::PricingRule",
    "AWS::Budgets::BudgetsAction",
    "AWS::CE::AnomalyMonitor",
    "AWS::CE::AnomalySubscription",
    "AWS::CE::CostCategory",
    "AWS::Comprehend::DocumentClassifier",
    "AWS::Comprehend::Flywheel",
    "AWS::CUR::ReportDefinition",
    "AWS::Cassandra::Keyspace",
    "AWS::Cassandra::Table",
    "AWS::CertificateManager::Account",
    "AWS::Chatbot::MicrosoftTeamsChannelConfiguration",
    "AWS::Chatbot::SlackChannelConfiguration",
    "AWS::CleanRooms::AnalysisTemplate",
    "AWS::CleanRooms::Collaboration",
    "AWS::CleanRooms::ConfiguredTable",
    "AWS::CleanRooms::ConfiguredTableAssociation",
    "AWS::CleanRooms::Membership",
    "AWS::CloudFormation::HookDefaultVersion",
    "AWS::CloudFormation::HookTypeConfig",
    "AWS::CloudFormation::HookVersion",
    "AWS::CloudFormation::ModuleDefaultVersion",
    "AWS::CloudFormation::ModuleVersion",
    "AWS::CloudFormation::PublicTypeVersion",
    "AWS::CloudFormation::Publisher",
    "AWS::CloudFormation::ResourceDefaultVersion",
    "AWS::CloudFormation::ResourceVersion",
    "AWS::CloudFormation::Stack",
    "AWS::CloudFormation::StackSet",
    "AWS::CloudFormation::TypeActivation",
    "AWS::CloudFront::CachePolicy",
    "AWS::CloudFront::ContinuousDeploymentPolicy",
    "AWS::CloudFront::CloudFrontOriginAccessIdentity",
    "AWS::CloudFront::Distribution",
    "AWS::CloudFront::Function",
    "AWS::CloudFront::KeyGroup",
    "AWS::CloudFront::KeyValueStore",
    "AWS::CloudFront::MonitoringSubscription",
    "AWS::CloudFront::OriginAccessControl",
    "AWS::CloudFront::OriginRequestPolicy",
    "AWS::CloudFront::PublicKey",
    "AWS::CloudFront::RealtimeLogConfig",
    "AWS::CloudFront::ResponseHeadersPolicy",
    "AWS::CloudTrail::Channel",
    "AWS::CloudTrail::EventDataStore",
    "AWS::CloudTrail::ResourcePolicy",
    "AWS::CloudTrail::Trail",
    "AWS::CloudWatch::Alarm",
    "AWS::CloudWatch::CompositeAlarm",
    "AWS::CloudWatch::MetricStream",
    "AWS::CodeArtifact::Domain",
    "AWS::CodeArtifact::Repository",
    "AWS::CodeBuild::Fleet",
    "AWS::CodeDeploy::Application",
    "AWS::CodeDeploy::DeploymentConfig",
    "AWS::CodeGuruProfiler::ProfilingGroup",
    "AWS::CodeGuruReviewer::RepositoryAssociation",
    "AWS::CodePipeline::CustomActionType",
    "AWS::CodeStarConnections::Connection",
    "AWS::CodeStarConnections::RepositoryLink",
    "AWS::CodeStarConnections::SyncConfiguration",
    "AWS::CodeStarNotifications::NotificationRule",
    "AWS::Cognito::IdentityPool",
    "AWS::Cognito::IdentityPoolPrincipalTag",
    "AWS::Cognito::IdentityPoolRoleAttachment",
    "AWS::Cognito::LogDeliveryConfiguration",
    "AWS::Cognito::UserPool",
    "AWS::Cognito::UserPoolClient",
    "AWS::Cognito::UserPoolGroup",
    "AWS::Cognito::UserPoolRiskConfigurationAttachment",
    "AWS::Cognito::UserPoolUser",
    "AWS::Cognito::UserPoolUserToGroupAttachment",
    "AWS::Config::AggregationAuthorization",
    "AWS::Config::ConfigRule",
    "AWS::Config::ConfigurationAggregator",
    "AWS::Config::ConformancePack",
    "AWS::Config::OrganizationConformancePack",
    "AWS::Config::StoredQuery",
    "AWS::Connect::ApprovedOrigin",
    "AWS::Connect::ContactFlow",
    "AWS::Connect::ContactFlowModule",
    "AWS::Connect::EvaluationForm",
    "AWS::Connect::HoursOfOperation",
    "AWS::Connect::Instance",
    "AWS::Connect::InstanceStorageConfig",
    "AWS::Connect::IntegrationAssociation",
    "AWS::Connect::PhoneNumber",
    "AWS::Connect::PredefinedAttribute",
    "AWS::Connect::Prompt",
    "AWS::Connect::Queue",
    "AWS::Connect::QuickConnect",
    "AWS::Connect::RoutingProfile",
    "AWS::Connect::Rule",
    "AWS::Connect::SecurityKey",
    "AWS::Connect::TaskTemplate",
    "AWS::Connect::TrafficDistributionGroup",
    "AWS::Connect::User",
    "AWS::Connect::UserHierarchyGroup",
    "AWS::Connect::View",
    "AWS::Connect::ViewVersion",
    "AWS::ConnectCampaigns::Campaign",
    "AWS::ControlTower::EnabledControl",
    "AWS::ControlTower::EnabledBaseline",
    "AWS::CustomerProfiles::CalculatedAttributeDefinition",
    "AWS::CustomerProfiles::Domain",
    "AWS::CustomerProfiles::EventStream",
    "AWS::CustomerProfiles::Integration",
    "AWS::ControlTower::LandingZone",
    "AWS::CustomerProfiles::ObjectType",
    "AWS::DMS::DataProvider",
    "AWS::DMS::InstanceProfile",
    "AWS::DMS::MigrationProject",
    "AWS::DMS::ReplicationConfig",
    "AWS::DataBrew::Dataset",
    "AWS::DataBrew::Job",
    "AWS::DataBrew::Project",
    "AWS::DataBrew::Recipe",
    "AWS::DataBrew::Ruleset",
    "AWS::DataBrew::Schedule",
    "AWS::DataPipeline::Pipeline",
    "AWS::DataSync::Agent",
    "AWS::DataSync::LocationAzureBlob",
    "AWS::DataSync::LocationEFS",
    "AWS::DataSync::LocationFSxLustre",
    "AWS::DataSync::LocationFSxONTAP",
    "AWS::DataSync::LocationFSxOpenZFS",
    "AWS::DataSync::LocationFSxWindows",
    "AWS::DataSync::LocationHDFS",
    "AWS::DataSync::LocationNFS",
    "AWS::DataSync::LocationObjectStorage",
    "AWS::DataSync::LocationS3",
    "AWS::DataSync::LocationSMB",
    "AWS::DataSync::StorageSystem",
    "AWS::DataSync::Task",
    "AWS::DataZone::DataSource",
    "AWS::DataZone::Domain",
    "AWS::DataZone::Environment",
    "AWS::DataZone::EnvironmentBlueprintConfiguration",
    "AWS::DataZone::EnvironmentProfile",
    "AWS::DataZone::Project",
    "AWS::DataZone::SubscriptionTarget",
    "AWS::Detective::OrganizationAdmin",
    "AWS::Detective::Graph",
    "AWS::Detective::MemberInvitation",
    "AWS::DevOpsGuru::NotificationChannel",
    "AWS::DevOpsGuru::ResourceCollection",
    "AWS::DeviceFarm::DevicePool",
    "AWS::DeviceFarm::InstanceProfile",
    "AWS::DeviceFarm::NetworkProfile",
    "AWS::DeviceFarm::Project",
    "AWS::DeviceFarm::TestGridProject",
    "AWS::DeviceFarm::VPCEConfiguration",
    "AWS::DevOpsGuru::LogAnomalyDetectionIntegration",
    "AWS::DirectoryService::SimpleAD",
    "AWS::DocDBElastic::Cluster",
    "AWS::DynamoDB::GlobalTable",
    "AWS::DynamoDB::Table",
    "AWS::EC2::CapacityReservation",
    "AWS::EC2::CapacityReservationFleet",
    "AWS::EC2::CarrierGateway",
    "AWS::EC2::CustomerGateway",
    "AWS::EC2::DHCPOptions",
    "AWS::EC2::EC2Fleet",
    "AWS::EC2::EgressOnlyInternetGateway",
    "AWS::EC2::EIP",
    "AWS::EC2::EIPAssociation",
    "AWS::EC2::EnclaveCertificateIamRoleAssociation",
    "AWS::EC2::FlowLog",
    "AWS::EC2::GatewayRouteTableAssociation",
    "AWS::EC2::Host",
    "AWS::EC2::Instance",
    "AWS::EC2::IPAM",
    "AWS::EC2::IPAMAllocation",
    "AWS::EC2::IPAMPool",
    "AWS::EC2::IPAMPoolCidr",
    "AWS::EC2::IPAMResourceDiscovery",
    "AWS::EC2::IPAMResourceDiscoveryAssociation",
    "AWS::EC2::IPAMScope",
    "AWS::EC2::Instance",
    "AWS::EC2::InstanceConnectEndpoint",
    "AWS::EC2::InternetGateway",
    "AWS::EC2::KeyPair",
    "AWS::EC2::LaunchTemplate",
    "AWS::EC2::LocalGatewayRoute",
    "AWS::EC2::LocalGatewayRouteTable",
    "AWS::EC2::LocalGatewayRouteTableVirtualInterfaceGroupAssociation",
    "AWS::EC2::LocalGatewayRouteTableVPCAssociation",
    "AWS::EC2::NatGateway",
    "AWS::EC2::NetworkAcl",
    "AWS::EC2::NetworkInsightsAccessScope",
    "AWS::EC2::NetworkInsightsAccessScopeAnalysis",
    "AWS::EC2::NetworkInsightsAnalysis",
    "AWS::EC2::NetworkInsightsPath",
    "AWS::EC2::NetworkInterface",
    "AWS::EC2::NetworkInterfaceAttachment",
    "AWS::EC2::NetworkPerformanceMetricSubscription",
    "AWS::EC2::PlacementGroup",
    "AWS::EC2::PrefixList",
    "AWS::EC2::Route",
    "AWS::EC2::RouteTable",
    "AWS::EC2::SecurityGroup",
    "AWS::EC2::SecurityGroupEgress",
    "AWS::EC2::SecurityGroupIngress",
    "AWS::EC2::SpotFleet",
    "AWS::EC2::Subnet",
    "AWS::EC2::SubnetCidrBlock",
    "AWS::EC2::SubnetNetworkAclAssociation",
    "AWS::EC2::SubnetRouteTableAssociation",
    "AWS::EC2::TransitGateway",
    "AWS::EC2::TransitGatewayAttachment",
    "AWS::EC2::TransitGatewayConnect",
    "AWS::EC2::TransitGatewayMulticastDomain",
    "AWS::EC2::TransitGatewayMulticastDomainAssociation",
    "AWS::EC2::TransitGatewayMulticastGroupMember",
    "AWS::EC2::TransitGatewayMulticastGroupSource",
    "AWS::EC2::TransitGatewayRouteTable",
    "AWS::EC2::TransitGatewayPeeringAttachment",
    "AWS::EC2::TransitGatewayVpcAttachment",
    "AWS::EC2::VPC",
    "AWS::EC2::VPCCidrBlock",
    "AWS::EC2::VPNConnection",
    "AWS::EC2::VPNConnectionRoute",
    "AWS::EC2::VPCDHCPOptionsAssociation",
    "AWS::EC2::VPCEndpoint",
    "AWS::EC2::VPCEndpointService",
    "AWS::EC2::VPCEndpointServicePermissions",
    "AWS::EC2::VPCPeeringConnection",
    "AWS::EC2::VPNGateway",
    "AWS::EC2::VerifiedAccessEndpoint",
    "AWS::EC2::VerifiedAccessGroup",
    "AWS::EC2::VerifiedAccessInstance",
    "AWS::EC2::VerifiedAccessTrustProvider",
    "AWS::EC2::Volume",
    "AWS::EC2::VolumeAttachment",
    "AWS::ECR::PublicRepository",
    "AWS::ECR::PullThroughCacheRule",
    "AWS::ECR::RegistryPolicy",
    "AWS::ECR::ReplicationConfiguration",
    "AWS::ECR::Repository",
    "AWS::ECS::CapacityProvider",
    "AWS::ECS::Cluster",
    "AWS::ECS::ClusterCapacityProviderAssociations",
    "AWS::ECS::PrimaryTaskSet",
    "AWS::ECS::Service",
    "AWS::ECS::TaskDefinition",
    "AWS::ECS::TaskSet",
    "AWS::EFS::AccessPoint",
    "AWS::EFS::FileSystem",
    "AWS::EKS::AccessEntry",
    "AWS::EKS::Addon",
    "AWS::EKS::Cluster",
    "AWS::EKS::FargateProfile",
    "AWS::EKS::IdentityProviderConfig",
    "AWS::EKS::Nodegroup",
    "AWS::EKS::PodIdentityAssociation",
    "AWS::ElastiCache::GlobalReplicationGroup",
    "AWS::ElastiCache::ServerlessCache",
    "AWS::ElastiCache::SubnetGroup",
    "AWS::ElastiCache::User",
    "AWS::ElastiCache::UserGroup",
    "AWS::ElasticBeanstalk::Application",
    "AWS::ElasticBeanstalk::ApplicationVersion",
    "AWS::ElasticBeanstalk::ConfigurationTemplate",
    "AWS::ElasticBeanstalk::Environment",
    "AWS::ElasticLoadBalancingV2::Listener",
    "AWS::ElasticLoadBalancingV2::ListenerRule",
    "AWS::ElasticLoadBalancingV2::LoadBalancer",
    "AWS::ElasticLoadBalancingV2::TrustStore",
    "AWS::ElasticLoadBalancingV2::TrustStoreRevocation",
    "AWS::EMR::SecurityConfiguration",
    "AWS::ElasticLoadBalancingV2::TargetGroup",
    "AWS::Elasticsearch::Domain",
    "AWS::EMR::Step",
    "AWS::EMR::Studio",
    "AWS::EMR::StudioSessionMapping",
    "AWS::EMR::WALWorkspace",
    "AWS::EMRContainers::VirtualCluster",
    "AWS::EMRServerless::Application",
    "AWS::EMRServerless::Application",
    "AWS::EntityResolution::IdMappingWorkflow",
    "AWS::EntityResolution::MatchingWorkflow",
    "AWS::EntityResolution::SchemaMapping",
    "AWS::EventSchemas::RegistryPolicy",
    "AWS::Events::ApiDestination",
    "AWS::Events::Archive",
    "AWS::Events::Connection",
    "AWS::Events::Endpoint",
    "AWS::Events::EventBus",
    "AWS::Events::Rule",
    "AWS::Evidently::Experiment",
    "AWS::Evidently::Feature",
    "AWS::Evidently::Launch",
    "AWS::Evidently::Project",
    "AWS::Evidently::Segment",
    "AWS::FIS::ExperimentTemplate",
    "AWS::FIS::TargetAccountConfiguration",
    "AWS::FMS::NotificationChannel",
    "AWS::FMS::Policy",
    "AWS::FMS::ResourceSet",
    "AWS::FinSpace::Environment",
    "AWS::Forecast::Dataset",
    "AWS::Forecast::DatasetGroup",
    "AWS::FraudDetector::Detector",
    "AWS::FraudDetector::EntityType",
    "AWS::FraudDetector::EventType",
    "AWS::FraudDetector::Label",
    "AWS::FraudDetector::List",
    "AWS::FraudDetector::Outcome",
    "AWS::FraudDetector::Variable",
    "AWS::FSx::DataRepositoryAssociation",
    "AWS::GameLift::Alias",
    "AWS::GameLift::Build",
    "AWS::GameLift::Fleet",
    "AWS::GameLift::GameSessionQueue",
    "AWS::GameLift::GameServerGroup",
    "AWS::GameLift::Location",
    "AWS::GameLift::MatchmakingConfiguration",
    "AWS::GameLift::MatchmakingRuleSet",
    "AWS::GameLift::Script",
    "AWS::GlobalAccelerator::Accelerator",
    "AWS::GlobalAccelerator::EndpointGroup",
    "AWS::GlobalAccelerator::Listener",
    "AWS::Glue::Registry",
    "AWS::Glue::Schema",
    "AWS::Glue::SchemaVersion",
    "AWS::Glue::SchemaVersionMetadata",
    "AWS::Grafana::Workspace",
    "AWS::GreengrassV2::ComponentVersion",
    "AWS::GroundStation::Config",
    "AWS::GroundStation::DataflowEndpointGroup",
    "AWS::GreengrassV2::Deployment",
    "AWS::GroundStation::MissionProfile",
    "AWS::GuardDuty::Detector",
    "AWS::GuardDuty::Filter",
    "AWS::GuardDuty::IPSet",
    "AWS::GuardDuty::Master",
    "AWS::GuardDuty::Member",
    "AWS::GuardDuty::ThreatIntelSet",
    "AWS::HealthImaging::Datastore",
    "AWS::HealthLake::FHIRDatastore",
    "AWS::IAM::InstanceProfile",
    "AWS::IAM::Group",
    "AWS::IAM::GroupPolicy",
    "AWS::IAM::ManagedPolicy",
    "AWS::IAM::OIDCProvider",
    "AWS::IAM::Role",
    "AWS::IAM::RolePolicy",
    "AWS::IAM::SAMLProvider",
    "AWS::IAM::ServerCertificate",
    "AWS::IAM::ServiceLinkedRole",
    "AWS::IAM::User",
    "AWS::IAM::UserPolicy",
    "AWS::IAM::VirtualMFADevice",
    "AWS::IVS::Channel",
    "AWS::IVS::PlaybackKeyPair",
    "AWS::IVS::RecordingConfiguration",
    "AWS::IVS::Stage",
    "AWS::IVS::StreamKey",
    "AWS::IVSChat::LoggingConfiguration",
    "AWS::IVSChat::Room",
    "AWS::IdentityStore::Group",
    "AWS::IdentityStore::GroupMembership",
    "AWS::ImageBuilder::Component",
    "AWS::ImageBuilder::ContainerRecipe",
    "AWS::ImageBuilder::DistributionConfiguration",
    "AWS::ImageBuilder::Image",
    "AWS::ImageBuilder::ImagePipeline",
    "AWS::ImageBuilder::ImageRecipe",
    "AWS::ImageBuilder::InfrastructureConfiguration",
    "AWS::ImageBuilder::LifecyclePolicy",
    "AWS::ImageBuilder::Workflow",
    "AWS::InternetMonitor::Monitor",
    "AWS::Inspector::AssessmentTarget",
    "AWS::Inspector::AssessmentTemplate",
    "AWS::Inspector::ResourceGroup",
    "AWS::InspectorV2::CisScanConfiguration",
    "AWS::InspectorV2::Filter",
    "AWS::IoT::AccountAuditConfiguration",
    "AWS::IoT::Authorizer",
    "AWS::IoT::BillingGroup",
    "AWS::IoT::CACertificate",
    "AWS::IoT::Certificate",
    "AWS::IoT::CertificateProvider",
    "AWS::IoT::CustomMetric",
    "AWS::IoT::Dimension",
    "AWS::IoT::DomainConfiguration",
    "AWS::IoT::FleetMetric",
    "AWS::IoT::JobTemplate",
    "AWS::IoT::Logging",
    "AWS::IoT::MitigationAction",
    "AWS::IoT::Policy",
    "AWS::IoT::ProvisioningTemplate",
    "AWS::IoT::ResourceSpecificLogging",
    "AWS::IoT::RoleAlias",
    "AWS::IoT::ScheduledAudit",
    "AWS::IoT::SecurityProfile",
    "AWS::IoT::SoftwarePackage",
    "AWS::IoT::SoftwarePackageVersion",
    "AWS::IoT::Thing",
    "AWS::IoT::ThingGroup",
    "AWS::IoT::ThingType",
    "AWS::IoT::TopicRule",
    "AWS::IoT::TopicRuleDestination",
    "AWS::IoTAnalytics::Channel",
    "AWS::IoTAnalytics::Dataset",
    "AWS::IoTAnalytics::Datastore",
    "AWS::IoTAnalytics::Pipeline",
    "AWS::IoTCoreDeviceAdvisor::SuiteDefinition",
    "AWS::IoTEvents::AlarmModel",
    "AWS::IoTEvents::DetectorModel",
    "AWS::IoTEvents::Input",
    "AWS::IoTFleetHub::Application",
    "AWS::IoTSiteWise::AccessPolicy",
    "AWS::IoTSiteWise::Asset",
    "AWS::IoTSiteWise::AssetModel",
    "AWS::IoTFleetWise::Campaign",
    "AWS::IoTSiteWise::Dashboard",
    "AWS::IoTFleetWise::DecoderManifest",
    "AWS::IoTFleetWise::Fleet",
    "AWS::IoTSiteWise::Gateway",
    "AWS::IoTFleetWise::ModelManifest",
    "AWS::IoTSiteWise::Portal",
    "AWS::IoTSiteWise::Project",
    "AWS::IoTFleetWise::SignalCatalog",
    "AWS::IoTFleetWise::Vehicle",
    "AWS::IoTTwinMaker::ComponentType",
    "AWS::IoTTwinMaker::Entity",
    "AWS::IoTTwinMaker::Scene",
    "AWS::IoTTwinMaker::SyncJob",
    "AWS::IoTTwinMaker::Workspace",
    "AWS::IoTWireless::Destination",
    "AWS::IoTWireless::DeviceProfile",
    "AWS::IoTWireless::FuotaTask",
    "AWS::IoTWireless::MulticastGroup",
    "AWS::IoTWireless::NetworkAnalyzerConfiguration",
    "AWS::IoTWireless::PartnerAccount",
    "AWS::IoTWireless::ServiceProfile",
    "AWS::IoTWireless::TaskDefinition",
    "AWS::IoTWireless::WirelessDevice",
    "AWS::IoTWireless::WirelessDeviceImportTask",
    "AWS::IoTWireless::WirelessGateway",
    "AWS::KMS::Alias",
    "AWS::KMS::Key",
    "AWS::KMS::ReplicaKey",
    "AWS::KafkaConnect::Connector",
    "AWS::Kendra::DataSource",
    "AWS::Kendra::Faq",
    "AWS::Kendra::Index",
    "AWS::KendraRanking::ExecutionPlan",
    "AWS::Kinesis::Stream",
    "AWS::KinesisAnalyticsV2::Application",
    "AWS::KinesisFirehose::DeliveryStream",
    "AWS::KinesisVideo::SignalingChannel",
    "AWS::KinesisVideo::Stream",
    "AWS::LakeFormation::DataCellsFilter",
    "AWS::LakeFormation::PrincipalPermissions",
    "AWS::LakeFormation::Tag",
    "AWS::LakeFormation::TagAssociation",
    "AWS::Lambda::CodeSigningConfig",
    "AWS::Lambda::EventInvokeConfig",
    "AWS::Lambda::Function",
    "AWS::Lambda::LayerVersion",
    "AWS::Lambda::LayerVersionPermission",
    "AWS::Lambda::Permission",
    "AWS::Lambda::Url",
    "AWS::Lambda::Version",
    "AWS::Lex::Bot",
    "AWS::Lex::BotAlias",
    "AWS::Lex::BotVersion",
    "AWS::Lex::ResourcePolicy",
    "AWS::LicenseManager::Grant",
    "AWS::LicenseManager::License",
    "AWS::Lightsail::Alarm",
    "AWS::Lightsail::Bucket",
    "AWS::Lightsail::Certificate",
    "AWS::Lightsail::Container",
    "AWS::Lightsail::Database",
    "AWS::Lightsail::Disk",
    "AWS::Lightsail::Distribution",
    "AWS::Lightsail::Instance",
    "AWS::Lightsail::LoadBalancer",
    "AWS::Lightsail::LoadBalancerTlsCertificate",
    "AWS::Lightsail::StaticIp",
    "AWS::Location::APIKey",
    "AWS::Location::GeofenceCollection",
    "AWS::Location::Map",
    "AWS::Location::PlaceIndex",
    "AWS::Location::RouteCalculator",
    "AWS::Location::Tracker",
    "AWS::Location::TrackerConsumer",
    "AWS::Logs::AccountPolicy",
    "AWS::Logs::Delivery",
    "AWS::Logs::DeliveryDestination",
    "AWS::Logs::DeliverySource",
    "AWS::Logs::Destination",
    "AWS::Logs::LogAnomalyDetector",
    "AWS::Logs::LogGroup",
    "AWS::Logs::LogStream",
    "AWS::Logs::MetricFilter",
    "AWS::Logs::QueryDefinition",
    "AWS::Logs::ResourcePolicy",
    "AWS::Logs::SubscriptionFilter",
    "AWS::LookoutEquipment::InferenceScheduler",
    "AWS::LookoutMetrics::Alert",
    "AWS::LookoutMetrics::AnomalyDetector",
    "AWS::LookoutVision::Project",
    "AWS::MSK::BatchScramSecret",
    "AWS::MSK::Cluster",
    "AWS::MSK::ClusterPolicy",
    "AWS::MSK::Configuration",
    "AWS::MSK::ServerlessCluster",
    "AWS::MSK::VpcConnection",
    "AWS::MWAA::Environment",
    "AWS::M2::Application",
    "AWS::M2::Environment",
    "AWS::Macie::AllowList",
    "AWS::Macie::CustomDataIdentifier",
    "AWS::Macie::FindingsFilter",
    "AWS::Macie::Session",
    "AWS::ManagedBlockchain::Accessor",
    "AWS::MediaConnect::Bridge",
    "AWS::MediaConnect::BridgeOutput",
    "AWS::MediaConnect::BridgeSource",
    "AWS::MediaConnect::Flow",
    "AWS::MediaConnect::FlowEntitlement",
    "AWS::MediaConnect::FlowOutput",
    "AWS::MediaConnect::FlowSource",
    "AWS::MediaConnect::FlowVpcInterface",
    "AWS::MediaConnect::Gateway",
    "AWS::MediaLive::Multiplex",
    "AWS::MediaLive::Multiplexprogram",
    "AWS::MediaPackage::Asset",
    "AWS::MediaPackage::Channel",
    "AWS::MediaPackage::OriginEndpoint",
    "AWS::MediaPackage::PackagingConfiguration",
    "AWS::MediaPackage::PackagingGroup",
    "AWS::MediaPackageV2::Channel",
    "AWS::MediaPackageV2::ChannelGroup",
    "AWS::MediaPackageV2::ChannelPolicy",
    "AWS::MediaPackageV2::OriginEndpoint",
    "AWS::MediaPackageV2::OriginEndpointPolicy",
    "AWS::MediaTailor::PlaybackConfiguration",
    "AWS::MediaTailor::LiveSource",
    "AWS::MediaTailor::VodSource",
    "AWS::MemoryDB::ACL",
    "AWS::MemoryDB::Cluster",
    "AWS::MemoryDB::ParameterGroup",
    "AWS::MemoryDB::SubnetGroup",
    "AWS::MemoryDB::User",
    "AWS::Neptune::DBCluster",
    "AWS::NeptuneGraph::Graph",
    "AWS::NeptuneGraph::Graph",
    "AWS::NetworkFirewall::Firewall",
    "AWS::NetworkFirewall::FirewallPolicy",
    "AWS::NetworkFirewall::LoggingConfiguration",
    "AWS::NetworkFirewall::RuleGroup",
    "AWS::NetworkFirewall::TLSInspectionConfiguration",
    "AWS::NetworkManager::ConnectAttachment",
    "AWS::NetworkManager::ConnectPeer",
    "AWS::NetworkManager::CoreNetwork",
    "AWS::NetworkManager::CustomerGatewayAssociation",
    "AWS::NetworkManager::Device",
    "AWS::NetworkManager::GlobalNetwork",
    "AWS::NetworkManager::Link",
    "AWS::NetworkManager::LinkAssociation",
    "AWS::NetworkManager::Site",
    "AWS::NetworkManager::SiteToSiteVpnAttachment",
    "AWS::NetworkManager::TransitGatewayPeering",
    "AWS::NetworkManager::TransitGatewayRegistration",
    "AWS::NetworkManager::TransitGatewayRouteTableAttachment",
    "AWS::NetworkManager::VpcAttachment",
    "AWS::NimbleStudio::LaunchProfile",
    "AWS::NimbleStudio::StreamingImage",
    "AWS::NimbleStudio::Studio",
    "AWS::NimbleStudio::StudioComponent",
    "AWS::Oam::Link",
    "AWS::Oam::Sink",
    "AWS::Omics::AnnotationStore",
    "AWS::Omics::ReferenceStore",
    "AWS::Omics::RunGroup",
    "AWS::Omics::SequenceStore",
    "AWS::Omics::VariantStore",
    "AWS::Omics::Workflow",
    "AWS::OpenSearchServerless::AccessPolicy",
    "AWS::OpenSearchServerless::Collection",
    "AWS::OpenSearchServerless::LifecyclePolicy",
    "AWS::OpenSearchServerless::SecurityConfig",
    "AWS::OpenSearchServerless::SecurityPolicy",
    "AWS::OpenSearchServerless::VpcEndpoint",
    "AWS::OpenSearchService::Domain",
    "AWS::OpsWorksCM::Server",
    "AWS::Organizations::Account",
    "AWS::Organizations::Organization",
    "AWS::Organizations::OrganizationalUnit",
    "AWS::Organizations::Policy",
    "AWS::Organizations::ResourcePolicy",
    "AWS::OSIS::Pipeline",
    "AWS::PCAConnectorAD::Connector",
    "AWS::PCAConnectorAD::DirectoryRegistration",
    "AWS::PCAConnectorAD::ServicePrincipalName",
    "AWS::PCAConnectorAD::Template",
    "AWS::PCAConnectorAD::TemplateGroupAccessControlEntry",
    "AWS::Panorama::ApplicationInstance",
    "AWS::Panorama::Package",
    "AWS::Panorama::PackageVersion",
    "AWS::Personalize::Dataset",
    "AWS::Personalize::DatasetGroup",
    "AWS::Personalize::Schema",
    "AWS::Personalize::Solution",
    "AWS::Pinpoint::InAppTemplate",
    "AWS::Pipes::Pipe",
    "AWS::Proton::EnvironmentAccountConnection",
    "AWS::Proton::EnvironmentTemplate",
    "AWS::Proton::ServiceTemplate",
    "AWS::QLDB::Stream",
    "AWS::QuickSight::Analysis",
    "AWS::QuickSight::Dashboard",
    "AWS::QuickSight::DataSet",
    "AWS::QuickSight::DataSource",
    "AWS::QuickSight::RefreshSchedule",
    "AWS::QuickSight::Template",
    "AWS::QuickSight::Theme",
    "AWS::QuickSight::Topic",
    "AWS::QuickSight::VPCConnection",
    "AWS::RAM::Permission",
    "AWS::RDS::CustomDBEngineVersion",
    "AWS::RDS::DBInstance",
    "AWS::RDS::DBCluster",
    "AWS::RDS::DBClusterParameterGroup",
    "AWS::RDS::DBParameterGroup",
    "AWS::RDS::DBProxy",
    "AWS::RDS::DBProxyEndpoint",
    "AWS::RDS::DBProxyTargetGroup",
    "AWS::RDS::DBSubnetGroup",
    "AWS::RDS::EventSubscription",
    "AWS::RDS::GlobalCluster",
    "AWS::RDS::Integration",
    "AWS::RDS::OptionGroup",
    "AWS::RUM::AppMonitor",
    "AWS::Redshift::Cluster",
    "AWS::Redshift::ClusterParameterGroup",
    "AWS::Redshift::ClusterSubnetGroup",
    "AWS::Redshift::EndpointAccess",
    "AWS::Redshift::EndpointAuthorization",
    "AWS::Redshift::EventSubscription",
    "AWS::Redshift::ScheduledAction",
    "AWS::RedshiftServerless::Namespace",
    "AWS::RedshiftServerless::Workgroup",
    "AWS::RefactorSpaces::Application",
    "AWS::RefactorSpaces::Environment",
    "AWS::RefactorSpaces::Route",
    "AWS::RefactorSpaces::Service",
    "AWS::Rekognition::Collection",
    "AWS::Rekognition::Project",
    "AWS::Rekognition::StreamProcessor",
    "AWS::ResilienceHub::App",
    "AWS::ResilienceHub::ResiliencyPolicy",
    "AWS::ResourceGroups::Group",
    "AWS::ResourceExplorer2::DefaultViewAssociation",
    "AWS::ResourceExplorer2::Index",
    "AWS::ResourceExplorer2::View",
    "AWS::RoboMaker::Fleet",
    "AWS::RoboMaker::Robot",
    "AWS::RoboMaker::RobotApplication",
    "AWS::RoboMaker::RobotApplicationVersion",
    "AWS::RoboMaker::SimulationApplication",
    "AWS::RoboMaker::SimulationApplicationVersion",
    "AWS::RolesAnywhere::CRL",
    "AWS::RolesAnywhere::Profile",
    "AWS::RolesAnywhere::TrustAnchor",
    "AWS::Route53::CidrCollection",
    "AWS::Route53::DNSSEC",
    "AWS::Route53::HealthCheck",
    "AWS::Route53::HostedZone",
    "AWS::Route53::KeySigningKey",
    "AWS::Route53RecoveryControl::Cluster",
    "AWS::Route53RecoveryControl::ControlPanel",
    "AWS::Route53RecoveryControl::RoutingControl",
    "AWS::Route53RecoveryControl::SafetyRule",
    "AWS::Route53RecoveryReadiness::Cell",
    "AWS::Route53RecoveryReadiness::ReadinessCheck",
    "AWS::Route53RecoveryReadiness::RecoveryGroup",
    "AWS::Route53RecoveryReadiness::ResourceSet",
    "AWS::Route53Resolver::FirewallDomainList",
    "AWS::Route53Resolver::FirewallRuleGroup",
    "AWS::Route53Resolver::FirewallRuleGroupAssociation",
    "AWS::Route53Resolver::OutpostResolver",
    "AWS::Route53Resolver::ResolverConfig",
    "AWS::Route53Resolver::ResolverDNSSECConfig",
    "AWS::Route53Resolver::ResolverQueryLoggingConfig",
    "AWS::Route53Resolver::ResolverQueryLoggingConfigAssociation",
    "AWS::Route53Resolver::ResolverRule",
    "AWS::Route53Resolver::ResolverRuleAssociation",
    "AWS::S3::AccessGrant",
    "AWS::S3::AccessGrantsInstance",
    "AWS::S3::AccessGrantsLocation",
    "AWS::S3::AccessPoint",
    "AWS::S3::Bucket",
    "AWS::S3::BucketPolicy",
    "AWS::S3::MultiRegionAccessPoint",
    "AWS::S3::MultiRegionAccessPointPolicy",
    "AWS::S3::StorageLens",
    "AWS::S3::StorageLensGroup",
    "AWS::S3Express::BucketPolicy",
    "AWS::S3Express::DirectoryBucket",
    "AWS::S3ObjectLambda::AccessPoint",
    "AWS::S3ObjectLambda::AccessPointPolicy",
    "AWS::S3Outposts::AccessPoint",
    "AWS::S3Outposts::Bucket",
    "AWS::S3Outposts::BucketPolicy",
    "AWS::S3Outposts::Endpoint",
    "AWS::Scheduler::Schedule",
    "AWS::SecretsManager::Secret",
    "AWS::SES::ConfigurationSet",
    "AWS::SES::ConfigurationSetEventDestination",
    "AWS::SES::ContactList",
    "AWS::SES::DedicatedIpPool",
    "AWS::SES::EmailIdentity",
    "AWS::SES::Template",
    "AWS::SES::VdmAttributes",
    "AWS::Shield::DRTAccess",
    "AWS::Shield::ProactiveEngagement",
    "AWS::Shield::Protection",
    "AWS::Shield::ProtectionGroup",
    "AWS::SimSpaceWeaver::Simulation",
    "AWS::SNS::Topic",
    "AWS::SNS::TopicInlinePolicy",
    "AWS::SQS::Queue",
    "AWS::SQS::QueueInlinePolicy",
    "AWS::SSM::Association",
    "AWS::SSM::Document",
    "AWS::SSM::Parameter",
    "AWS::SSM::ResourceDataSync",
    "AWS::SSMContacts::Contact",
    "AWS::SSMContacts::ContactChannel",
    "AWS::SSMContacts::Plan",
    "AWS::SSMIncidents::ReplicationSet",
    "AWS::SSMIncidents::ResponsePlan",
    "AWS::SSMContacts::Rotation",
    "AWS::SSO::Assignment",
    "AWS::SSO::InstanceAccessControlAttributeConfiguration",
    "AWS::SSO::PermissionSet",
    "AWS::SageMaker::App",
    "AWS::SageMaker::AppImageConfig",
    "AWS::SageMaker::DataQualityJobDefinition",
    "AWS::SageMaker::Device",
    "AWS::SageMaker::DeviceFleet",
    "AWS::SageMaker::Domain",
    "AWS::SageMaker::FeatureGroup",
    "AWS::SageMaker::Image",
    "AWS::SageMaker::ImageVersion",
    "AWS::SageMaker::InferenceComponent",
    "AWS::SageMaker::InferenceExperiment",
    "AWS::SageMaker::ModelBiasJobDefinition",
    "AWS::SageMaker::ModelCard",
    "AWS::SageMaker::ModelExplainabilityJobDefinition",
    "AWS::SageMaker::ModelPackage",
    "AWS::SageMaker::ModelPackageGroup",
    "AWS::SageMaker::ModelQualityJobDefinition",
    "AWS::SageMaker::MonitoringSchedule",
    "AWS::SageMaker::Pipeline",
    "AWS::SageMaker::Project",
    "AWS::SageMaker::Space",
    "AWS::SageMaker::UserProfile",
    "AWS::Scheduler::Schedule",
    "AWS::Scheduler::ScheduleGroup",
    "AWS::SecurityHub::AutomationRule",
    "AWS::SecurityHub::DelegatedAdmin",
    "AWS::SecurityHub::Hub",
    "AWS::SecurityHub::Insight",
    "AWS::SecurityHub::ProductSubscription",
    "AWS::SecurityHub::Standard",
    "AWS::ServiceCatalog::CloudFormationProvisionedProduct",
    "AWS::ServiceCatalog::ServiceAction",
    "AWS::ServiceCatalog::ServiceActionAssociation",
    "AWS::ServiceCatalogAppRegistry::Application",
    "AWS::ServiceCatalogAppRegistry::AttributeGroup",
    "AWS::ServiceCatalogAppRegistry::AttributeGroupAssociation",
    "AWS::ServiceCatalogAppRegistry::ResourceAssociation",
    "AWS::Signer::ProfilePermission",
    "AWS::Signer::SigningProfile",
    "AWS::StepFunctions::Activity",
    "AWS::StepFunctions::StateMachine",
    "AWS::StepFunctions::StateMachineAlias",
    "AWS::SupportApp::AccountAlias",
    "AWS::SupportApp::SlackChannelConfiguration",
    "AWS::SupportApp::SlackWorkspaceConfiguration",
    "AWS::Synthetics::Canary",
    "AWS::Synthetics::Group",
    "AWS::SystemsManagerSAP::Application",
    "AWS::Timestream::Database",
    "AWS::Timestream::ScheduledQuery",
    "AWS::Timestream::Table",
    "AWS::Transfer::Agreement",
    "AWS::Transfer::Certificate",
    "AWS::Transfer::Connector",
    "AWS::Transfer::Profile",
    "AWS::Transfer::Workflow",
    "AWS::VerifiedPermissions::IdentitySource",
    "AWS::VerifiedPermissions::Policy",
    "AWS::VerifiedPermissions::PolicyStore",
    "AWS::VerifiedPermissions::PolicyTemplate",
    "AWS::VoiceID::Domain",
    "AWS::VpcLattice::AccessLogSubscription",
    "AWS::VpcLattice::AuthPolicy",
    "AWS::VpcLattice::Listener",
    "AWS::VpcLattice::ResourcePolicy",
    "AWS::VpcLattice::Rule",
    "AWS::VpcLattice::Service",
    "AWS::VpcLattice::ServiceNetwork",
    "AWS::VpcLattice::ServiceNetworkServiceAssociation",
    "AWS::VpcLattice::ServiceNetworkVpcAssociation",
    "AWS::VpcLattice::TargetGroup",
    "AWS::WAFv2::IPSet",
    "AWS::WAFv2::LoggingConfiguration",
    "AWS::WAFv2::RegexPatternSet",
    "AWS::WAFv2::RuleGroup",
    "AWS::WAFv2::WebACL",
    "AWS::WAFv2::WebACLAssociation",
    "AWS::Wisdom::Assistant",
    "AWS::Wisdom::AssistantAssociation",
    "AWS::Wisdom::KnowledgeBase",
    "AWS::WorkSpaces::ConnectionAlias",
    "AWS::WorkSpacesThinClient::Environment",
    "AWS::WorkSpacesWeb::BrowserSettings",
    "AWS::WorkSpacesWeb::IdentityProvider",
    "AWS::WorkSpacesWeb::IpAccessSettings",
    "AWS::WorkSpacesWeb::NetworkSettings",
    "AWS::WorkSpacesWeb::Portal",
    "AWS::WorkSpacesWeb::TrustStore",
    "AWS::WorkSpacesWeb::UserAccessLoggingSettings",
    "AWS::WorkSpacesWeb::UserSettings",
    "AWS::XRay::Group",
    "AWS::XRay::ResourcePolicy",
    "AWS::XRay::SamplingRule",
];
