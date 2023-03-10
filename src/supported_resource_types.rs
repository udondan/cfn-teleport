pub const SUPPORTED_RESOURCE_TYPES: [&str; 585] = [
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
    "AWS::ApiGateway::Stage",
    "AWS::ApiGateway::UsagePlan",
    "AWS::ApiGateway::UsagePlanKey",
    "AWS::ApiGatewayV2::VpcLink",
    "AWS::AppFlow::ConnectorProfile",
    "AWS::AppFlow::Flow",
    "AWS::AppIntegrations::DataIntegration",
    "AWS::AppIntegrations::EventIntegration",
    "AWS::AppRunner::ObservabilityConfiguration",
    "AWS::AppRunner::Service",
    "AWS::AppRunner::VpcConnector",
    "AWS::AppStream::AppBlock",
    "AWS::AppStream::Application",
    "AWS::AppStream::ApplicationEntitlementAssociation",
    "AWS::AppStream::ApplicationFleetAssociation",
    "AWS::AppStream::DirectoryConfig",
    "AWS::AppStream::Entitlement",
    "AWS::AppStream::ImageBuilder",
    "AWS::AppSync::DomainName",
    "AWS::AppSync::DomainNameApiAssociation",
    "AWS::ApplicationInsights::Application",
    "AWS::Athena::DataCatalog",
    "AWS::Athena::NamedQuery",
    "AWS::Athena::PreparedStatement",
    "AWS::Athena::WorkGroup",
    "AWS::AuditManager::Assessment",
    "AWS::AutoScaling::LaunchConfiguration",
    "AWS::AutoScaling::LifecycleHook",
    "AWS::AutoScaling::ScalingPolicy",
    "AWS::AutoScaling::WarmPool",
    "AWS::Backup::BackupPlan",
    "AWS::Backup::BackupSelection",
    "AWS::Backup::BackupVault",
    "AWS::Backup::Framework",
    "AWS::Backup::ReportPlan",
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
    "AWS::CUR::ReportDefinition",
    "AWS::Cassandra::Keyspace",
    "AWS::Cassandra::Table",
    "AWS::CertificateManager::Account",
    "AWS::Chatbot::SlackChannelConfiguration",
    "AWS::CloudFormation::HookDefaultVersion",
    "AWS::CloudFormation::HookTypeConfig",
    "AWS::CloudFormation::HookVersion",
    "AWS::CloudFormation::ModuleDefaultVersion",
    "AWS::CloudFormation::ModuleVersion",
    "AWS::CloudFormation::PublicTypeVersion",
    "AWS::CloudFormation::Publisher",
    "AWS::CloudFormation::ResourceDefaultVersion",
    "AWS::CloudFormation::ResourceVersion",
    "AWS::CloudFormation::StackSet",
    "AWS::CloudFormation::TypeActivation",
    "AWS::CloudFront::CachePolicy",
    "AWS::CloudFront::CloudFrontOriginAccessIdentity",
    "AWS::CloudFront::Distribution",
    "AWS::CloudFront::Function",
    "AWS::CloudFront::KeyGroup",
    "AWS::CloudFront::OriginRequestPolicy",
    "AWS::CloudFront::PublicKey",
    "AWS::CloudFront::RealtimeLogConfig",
    "AWS::CloudFront::ResponseHeadersPolicy",
    "AWS::CloudTrail::EventDataStore",
    "AWS::CloudTrail::Trail",
    "AWS::CloudWatch::CompositeAlarm",
    "AWS::CloudWatch::MetricStream",
    "AWS::CodeArtifact::Domain",
    "AWS::CodeArtifact::Repository",
    "AWS::CodeGuruProfiler::ProfilingGroup",
    "AWS::CodeGuruReviewer::RepositoryAssociation",
    "AWS::CodeStarConnections::Connection",
    "AWS::CodeStarNotifications::NotificationRule",
    "AWS::Config::AggregationAuthorization",
    "AWS::Config::ConfigurationAggregator",
    "AWS::Config::ConformancePack",
    "AWS::Config::OrganizationConformancePack",
    "AWS::Config::StoredQuery",
    "AWS::Connect::ContactFlow",
    "AWS::Connect::ContactFlowModule",
    "AWS::Connect::HoursOfOperation",
    "AWS::Connect::PhoneNumber",
    "AWS::Connect::QuickConnect",
    "AWS::Connect::TaskTemplate",
    "AWS::Connect::User",
    "AWS::Connect::UserHierarchyGroup",
    "AWS::ConnectCampaigns::Campaign",
    "AWS::CustomerProfiles::Domain",
    "AWS::CustomerProfiles::Integration",
    "AWS::CustomerProfiles::ObjectType",
    "AWS::DataBrew::Dataset",
    "AWS::DataBrew::Job",
    "AWS::DataBrew::Project",
    "AWS::DataBrew::Recipe",
    "AWS::DataBrew::Ruleset",
    "AWS::DataBrew::Schedule",
    "AWS::DataSync::Agent",
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
    "AWS::DataSync::Task",
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
    "AWS::DynamoDB::GlobalTable",
    "AWS::DynamoDB::Table",
    "AWS::EC2::CapacityReservation",
    "AWS::EC2::CapacityReservationFleet",
    "AWS::EC2::CarrierGateway",
    "AWS::EC2::CustomerGateway",
    "AWS::EC2::DHCPOptions",
    "AWS::EC2::EC2Fleet",
    "AWS::EC2::EgressOnlyInternetGateway",
    "AWS::EC2::EnclaveCertificateIamRoleAssociation",
    "AWS::EC2::FlowLog",
    "AWS::EC2::GatewayRouteTableAssociation",
    "AWS::EC2::Host",
    "AWS::EC2::IPAM",
    "AWS::EC2::IPAMAllocation",
    "AWS::EC2::IPAMPool",
    "AWS::EC2::IPAMScope",
    "AWS::EC2::Instance",
    "AWS::EC2::InternetGateway",
    "AWS::EC2::KeyPair",
    "AWS::EC2::LocalGatewayRoute",
    "AWS::EC2::LocalGatewayRouteTableVPCAssociation",
    "AWS::EC2::NatGateway",
    "AWS::EC2::NetworkAcl",
    "AWS::EC2::NetworkInsightsAccessScope",
    "AWS::EC2::NetworkInsightsAccessScopeAnalysis",
    "AWS::EC2::NetworkInsightsAnalysis",
    "AWS::EC2::NetworkInsightsPath",
    "AWS::EC2::NetworkInterface",
    "AWS::EC2::PlacementGroup",
    "AWS::EC2::PrefixList",
    "AWS::EC2::RouteTable",
    "AWS::EC2::SecurityGroup",
    "AWS::EC2::SpotFleet",
    "AWS::EC2::Subnet",
    "AWS::EC2::SubnetNetworkAclAssociation",
    "AWS::EC2::SubnetRouteTableAssociation",
    "AWS::EC2::TransitGateway",
    "AWS::EC2::TransitGatewayAttachment",
    "AWS::EC2::TransitGatewayConnect",
    "AWS::EC2::TransitGatewayMulticastDomain",
    "AWS::EC2::TransitGatewayMulticastDomainAssociation",
    "AWS::EC2::TransitGatewayMulticastGroupMember",
    "AWS::EC2::TransitGatewayMulticastGroupSource",
    "AWS::EC2::TransitGatewayPeeringAttachment",
    "AWS::EC2::TransitGatewayVpcAttachment",
    "AWS::EC2::VPC",
    "AWS::EC2::VPCDHCPOptionsAssociation",
    "AWS::EC2::VPCPeeringConnection",
    "AWS::EC2::VPNGateway",
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
    "AWS::EFS::MountTarget",
    "AWS::EKS::Addon",
    "AWS::EKS::Cluster",
    "AWS::EKS::FargateProfile",
    "AWS::EKS::IdentityProviderConfig",
    "AWS::EKS::Nodegroup",
    "AWS::EMR::Studio",
    "AWS::EMR::StudioSessionMapping",
    "AWS::EMRContainers::VirtualCluster",
    "AWS::EMRServerless::Application",
    "AWS::ElastiCache::GlobalReplicationGroup",
    "AWS::ElastiCache::User",
    "AWS::ElastiCache::UserGroup",
    "AWS::ElasticBeanstalk::Application",
    "AWS::ElasticLoadBalancingV2::Listener",
    "AWS::ElasticLoadBalancingV2::ListenerRule",
    "AWS::EventSchemas::RegistryPolicy",
    "AWS::Events::ApiDestination",
    "AWS::Events::Archive",
    "AWS::Events::Connection",
    "AWS::Events::Endpoint",
    "AWS::Evidently::Experiment",
    "AWS::Evidently::Feature",
    "AWS::Evidently::Launch",
    "AWS::Evidently::Project",
    "AWS::Evidently::Segment",
    "AWS::FIS::ExperimentTemplate",
    "AWS::FMS::NotificationChannel",
    "AWS::FMS::Policy",
    "AWS::FinSpace::Environment",
    "AWS::Forecast::Dataset",
    "AWS::Forecast::DatasetGroup",
    "AWS::FraudDetector::Detector",
    "AWS::FraudDetector::EntityType",
    "AWS::FraudDetector::EventType",
    "AWS::FraudDetector::Label",
    "AWS::FraudDetector::Outcome",
    "AWS::FraudDetector::Variable",
    "AWS::GameLift::Alias",
    "AWS::GameLift::Fleet",
    "AWS::GameLift::GameServerGroup",
    "AWS::GlobalAccelerator::Accelerator",
    "AWS::GlobalAccelerator::EndpointGroup",
    "AWS::GlobalAccelerator::Listener",
    "AWS::Glue::Registry",
    "AWS::Glue::Schema",
    "AWS::Glue::SchemaVersion",
    "AWS::Glue::SchemaVersionMetadata",
    "AWS::GreengrassV2::ComponentVersion",
    "AWS::GroundStation::Config",
    "AWS::GroundStation::DataflowEndpointGroup",
    "AWS::GroundStation::MissionProfile",
    "AWS::HealthLake::FHIRDatastore",
    "AWS::IAM::InstanceProfile",
    "AWS::IAM::OIDCProvider",
    "AWS::IAM::Role",
    "AWS::IAM::SAMLProvider",
    "AWS::IAM::ServerCertificate",
    "AWS::IAM::VirtualMFADevice",
    "AWS::IVS::Channel",
    "AWS::IVS::PlaybackKeyPair",
    "AWS::IVS::RecordingConfiguration",
    "AWS::IVS::StreamKey",
    "AWS::ImageBuilder::Component",
    "AWS::ImageBuilder::ContainerRecipe",
    "AWS::ImageBuilder::DistributionConfiguration",
    "AWS::ImageBuilder::Image",
    "AWS::ImageBuilder::ImagePipeline",
    "AWS::ImageBuilder::ImageRecipe",
    "AWS::ImageBuilder::InfrastructureConfiguration",
    "AWS::Inspector::AssessmentTarget",
    "AWS::Inspector::AssessmentTemplate",
    "AWS::Inspector::ResourceGroup",
    "AWS::InspectorV2::Filter",
    "AWS::IoT::AccountAuditConfiguration",
    "AWS::IoT::Authorizer",
    "AWS::IoT::CACertificate",
    "AWS::IoT::Certificate",
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
    "AWS::IoTSiteWise::Dashboard",
    "AWS::IoTSiteWise::Gateway",
    "AWS::IoTSiteWise::Portal",
    "AWS::IoTSiteWise::Project",
    "AWS::IoTTwinMaker::ComponentType",
    "AWS::IoTTwinMaker::Entity",
    "AWS::IoTTwinMaker::Scene",
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
    "AWS::IoTWireless::WirelessGateway",
    "AWS::KMS::Alias",
    "AWS::KMS::Key",
    "AWS::KMS::ReplicaKey",
    "AWS::KafkaConnect::Connector",
    "AWS::Kendra::DataSource",
    "AWS::Kendra::Faq",
    "AWS::Kendra::Index",
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
    "AWS::Lambda::Function",
    "AWS::Lambda::Url",
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
    "AWS::Location::GeofenceCollection",
    "AWS::Location::Map",
    "AWS::Location::PlaceIndex",
    "AWS::Location::RouteCalculator",
    "AWS::Location::Tracker",
    "AWS::Location::TrackerConsumer",
    "AWS::Logs::LogGroup",
    "AWS::Logs::MetricFilter",
    "AWS::Logs::QueryDefinition",
    "AWS::Logs::ResourcePolicy",
    "AWS::LookoutEquipment::InferenceScheduler",
    "AWS::LookoutMetrics::Alert",
    "AWS::LookoutMetrics::AnomalyDetector",
    "AWS::LookoutVision::Project",
    "AWS::MSK::BatchScramSecret",
    "AWS::MSK::Cluster",
    "AWS::MSK::Configuration",
    "AWS::MSK::ServerlessCluster",
    "AWS::MWAA::Environment",
    "AWS::Macie::CustomDataIdentifier",
    "AWS::Macie::FindingsFilter",
    "AWS::Macie::Session",
    "AWS::MediaConnect::Flow",
    "AWS::MediaConnect::FlowEntitlement",
    "AWS::MediaConnect::FlowOutput",
    "AWS::MediaConnect::FlowSource",
    "AWS::MediaConnect::FlowVpcInterface",
    "AWS::MediaPackage::Asset",
    "AWS::MediaPackage::Channel",
    "AWS::MediaPackage::OriginEndpoint",
    "AWS::MediaPackage::PackagingConfiguration",
    "AWS::MediaPackage::PackagingGroup",
    "AWS::MediaTailor::PlaybackConfiguration",
    "AWS::MemoryDB::ACL",
    "AWS::MemoryDB::Cluster",
    "AWS::MemoryDB::ParameterGroup",
    "AWS::MemoryDB::SubnetGroup",
    "AWS::MemoryDB::User",
    "AWS::NetworkFirewall::Firewall",
    "AWS::NetworkFirewall::FirewallPolicy",
    "AWS::NetworkFirewall::LoggingConfiguration",
    "AWS::NetworkFirewall::RuleGroup",
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
    "AWS::NetworkManager::TransitGatewayRegistration",
    "AWS::NetworkManager::VpcAttachment",
    "AWS::NimbleStudio::LaunchProfile",
    "AWS::NimbleStudio::StreamingImage",
    "AWS::NimbleStudio::Studio",
    "AWS::NimbleStudio::StudioComponent",
    "AWS::OpenSearchService::Domain",
    "AWS::OpsWorksCM::Server",
    "AWS::Panorama::ApplicationInstance",
    "AWS::Panorama::Package",
    "AWS::Panorama::PackageVersion",
    "AWS::Personalize::Dataset",
    "AWS::Personalize::DatasetGroup",
    "AWS::Personalize::Schema",
    "AWS::Personalize::Solution",
    "AWS::Pinpoint::InAppTemplate",
    "AWS::QLDB::Stream",
    "AWS::QuickSight::Analysis",
    "AWS::QuickSight::Dashboard",
    "AWS::QuickSight::DataSet",
    "AWS::QuickSight::DataSource",
    "AWS::QuickSight::Template",
    "AWS::QuickSight::Theme",
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
    "AWS::RDS::OptionGroup",
    "AWS::RUM::AppMonitor",
    "AWS::Redshift::Cluster",
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
    "AWS::Route53Resolver::ResolverConfig",
    "AWS::Route53Resolver::ResolverDNSSECConfig",
    "AWS::Route53Resolver::ResolverQueryLoggingConfig",
    "AWS::Route53Resolver::ResolverQueryLoggingConfigAssociation",
    "AWS::Route53Resolver::ResolverRule",
    "AWS::Route53Resolver::ResolverRuleAssociation",
    "AWS::S3::AccessPoint",
    "AWS::S3::Bucket",
    "AWS::S3::MultiRegionAccessPoint",
    "AWS::S3::MultiRegionAccessPointPolicy",
    "AWS::S3::StorageLens",
    "AWS::S3ObjectLambda::AccessPoint",
    "AWS::S3ObjectLambda::AccessPointPolicy",
    "AWS::S3Outposts::AccessPoint",
    "AWS::S3Outposts::Bucket",
    "AWS::S3Outposts::BucketPolicy",
    "AWS::S3Outposts::Endpoint",
    "AWS::SES::ConfigurationSet",
    "AWS::SES::ConfigurationSetEventDestination",
    "AWS::SES::ContactList",
    "AWS::SES::DedicatedIpPool",
    "AWS::SES::EmailIdentity",
    "AWS::SES::Template",
    "AWS::SQS::Queue",
    "AWS::SSM::Association",
    "AWS::SSM::Document",
    "AWS::SSM::ResourceDataSync",
    "AWS::SSMContacts::Contact",
    "AWS::SSMContacts::ContactChannel",
    "AWS::SSMIncidents::ReplicationSet",
    "AWS::SSMIncidents::ResponsePlan",
    "AWS::SSO::Assignment",
    "AWS::SSO::InstanceAccessControlAttributeConfiguration",
    "AWS::SageMaker::App",
    "AWS::SageMaker::AppImageConfig",
    "AWS::SageMaker::DataQualityJobDefinition",
    "AWS::SageMaker::Device",
    "AWS::SageMaker::DeviceFleet",
    "AWS::SageMaker::Domain",
    "AWS::SageMaker::FeatureGroup",
    "AWS::SageMaker::Image",
    "AWS::SageMaker::ImageVersion",
    "AWS::SageMaker::ModelBiasJobDefinition",
    "AWS::SageMaker::ModelExplainabilityJobDefinition",
    "AWS::SageMaker::ModelPackage",
    "AWS::SageMaker::ModelPackageGroup",
    "AWS::SageMaker::ModelQualityJobDefinition",
    "AWS::SageMaker::MonitoringSchedule",
    "AWS::SageMaker::Pipeline",
    "AWS::SageMaker::Project",
    "AWS::SageMaker::UserProfile",
    "AWS::ServiceCatalog::CloudFormationProvisionedProduct",
    "AWS::ServiceCatalog::ServiceAction",
    "AWS::ServiceCatalog::ServiceActionAssociation",
    "AWS::ServiceCatalogAppRegistry::Application",
    "AWS::ServiceCatalogAppRegistry::AttributeGroup",
    "AWS::ServiceCatalogAppRegistry::AttributeGroupAssociation",
    "AWS::ServiceCatalogAppRegistry::ResourceAssociation",
    "AWS::Signer::ProfilePermission",
    "AWS::Signer::SigningProfile",
    "AWS::SSO::PermissionSet",
    "AWS::StepFunctions::Activity",
    "AWS::StepFunctions::StateMachine",
    "AWS::Synthetics::Canary",
    "AWS::Synthetics::Group",
    "AWS::Timestream::Database",
    "AWS::Timestream::ScheduledQuery",
    "AWS::Timestream::Table",
    "AWS::Transfer::Workflow",
    "AWS::VoiceID::Domain",
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
    "AWS::XRay::Group",
    "AWS::XRay::SamplingRule",
];