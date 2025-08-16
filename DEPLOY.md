# 🚀 QBot AWS Deployment Guide

This guide provides detailed instructions for deploying QBot Discord bot to AWS using Amazon ECS (Elastic Container Service) with EC2 instances.

## 📋 Table of Contents

- [Prerequisites](#prerequisites)
- [Architecture Overview](#architecture-overview)
- [Quick Start](#quick-start)
- [Manual Deployment](#manual-deployment)
- [Configuration](#configuration)
- [Monitoring & Troubleshooting](#monitoring--troubleshooting)
- [Scaling & Management](#scaling--management)
- [Cost Estimation](#cost-estimation)
- [Security Considerations](#security-considerations)
- [Cleanup](#cleanup)
- [FAQ](#faq)

## 🔧 Prerequisites

### Required Tools

1. **AWS CLI v2** - [Installation Guide](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html)
   ```bash
   aws --version
   # Should show: aws-cli/2.x.x or higher
   ```

2. **jq** - JSON processor for scripts
   ```bash
   # macOS
   brew install jq
   
   # Ubuntu/Debian
   sudo apt-get install jq
   
   # Amazon Linux/RHEL/CentOS
   sudo yum install jq
   ```

3. **Discord Bot Token** - [Create a Discord Application](https://discord.com/developers/applications)

### AWS Account Setup

1. **AWS Account** with appropriate permissions
2. **AWS CLI configured** with credentials:
   ```bash
   aws configure
   ```
   
3. **Required AWS Permissions** - Your AWS user/role needs:
   - CloudFormation full access
   - ECS full access
   - EC2 full access (for VPC, security groups)
   - IAM role creation and management
   - EFS full access
   - Systems Manager Parameter Store access
   - CloudWatch Logs access

## 🏗️ Architecture Overview

QBot runs on AWS using the following architecture:

```
┌─────────────────────────────────────────────────────────────┐
│                        AWS Cloud                            │
│  ┌─────────────────────────────────────────────────────────┐ │
│  │                    VPC                                  │ │
│  │  ┌─────────────────┐    ┌─────────────────────────────┐ │ │
│  │  │  Public Subnet  │    │     Private Subnet          │ │ │
│  │  │                 │    │  ┌─────────────────────────┐ │ │ │
│  │  │   NAT Gateway   │    │  │    EC2 Instances        │ │ │ │
│  │  │                 │    │  │  ┌─────────┬─────────┐  │ │ │ │
│  │  └─────────────────┘    │  │  │ QBot    │ Ollama  │  │ │ │ │
│  │                         │  │  │Container│Container│  │ │ │ │
│  │  ┌─────────────────┐    │  │  └─────────┴─────────┘  │ │ │ │
│  │  │ Internet Gateway│    │  └─────────────────────────┘ │ │ │
│  │  └─────────────────┘    │              │               │ │ │
│  └─────────────────────────│──────────────│───────────────┘ │
│                            │              │                 │
│  ┌─────────────────────────┼──────────────┼───────────────┐ │
│  │       EFS File System   │              │               │ │
│  │    (Ollama Models)      │◄─────────────┘               │ │
│  └─────────────────────────┼───────────────────────────────┘ │
│                            │                                 │
│  ┌─────────────────────────┼───────────────────────────────┐ │
│  │  Parameter Store        │                               │ │
│  │  (Discord Token)        │                               │ │
│  └─────────────────────────┼───────────────────────────────┘ │
└────────────────────────────┼─────────────────────────────────┘
                             │
                 ┌───────────┼────────────┐
                 │     Discord API        │
                 └────────────────────────┘
```

### Key Components

- **ECS with EC2**: Container execution on EC2 instances with auto-scaling
- **VPC**: Isolated network with public/private subnets
- **EFS**: Persistent storage for Ollama models
- **Systems Manager**: Secure storage for Discord token
- **CloudWatch**: Logging and monitoring
- **NAT Gateway**: Outbound internet access for containers
- **Auto Scaling Group**: Manages EC2 instance capacity automatically

## 🚀 Quick Start

The fastest way to deploy QBot is using the automated deployment script:

### 1. Get Your Discord Token

1. Go to [Discord Developer Portal](https://discord.com/developers/applications)
2. Create a new application or use an existing one
3. Go to the "Bot" section
4. Copy the bot token (keep it secure!)

### 2. Deploy with One Command

```bash
# Clone the repository
git clone https://github.com/ThatXliner/qbot.git
cd qbot

# Set your Discord token
export DISCORD_TOKEN="your_discord_bot_token_here"

# Deploy to AWS (default region: us-east-1)
./aws/scripts/deploy.sh

# Or specify a different region
./aws/scripts/deploy.sh --region us-west-2
```

### 3. Monitor Deployment

The script will:
1. ✅ Check dependencies and AWS credentials
2. ✅ Deploy networking and storage infrastructure
3. ✅ Register ECS task definition
4. ✅ Deploy the ECS service
5. ✅ Wait for the service to become stable

**Deployment typically takes 10-15 minutes** due to NAT Gateway and EFS setup.

### 4. Verify Deployment

After deployment, check the AWS Console:
- **ECS Service**: https://console.aws.amazon.com/ecs/home#/clusters/qbot-cluster/services
- **CloudWatch Logs**: https://console.aws.amazon.com/cloudwatch/home#logsV2:log-groups/log-group/%2Fecs%2Fqbot

## 🔧 Manual Deployment

For more control or understanding, you can deploy manually:

### Step 1: Deploy Infrastructure

```bash
aws cloudformation deploy \
  --template-file aws/cloudformation/infrastructure.yml \
  --stack-name qbot-infrastructure \
  --parameter-overrides DiscordToken="$DISCORD_TOKEN" \
  --capabilities CAPABILITY_NAMED_IAM \
  --region us-east-1
```

### Step 2: Register Task Definition

```bash
# Get stack outputs
ACCOUNT_ID=$(aws sts get-caller-identity --query Account --output text)
EFS_ID=$(aws cloudformation describe-stacks --stack-name qbot-infrastructure \
  --query 'Stacks[0].Outputs[?OutputKey==`EFSFileSystem`].OutputValue' --output text)
EFS_ACCESS_POINT=$(aws cloudformation describe-stacks --stack-name qbot-infrastructure \
  --query 'Stacks[0].Outputs[?OutputKey==`EFSAccessPoint`].OutputValue' --output text)

# Update task definition template
sed -e "s/{AWS_ACCOUNT_ID}/$ACCOUNT_ID/g" \
    -e "s/{AWS_REGION}/us-east-1/g" \
    -e "s/{EFS_FILE_SYSTEM_ID}/$EFS_ID/g" \
    -e "s/{EFS_ACCESS_POINT_ID}/$EFS_ACCESS_POINT/g" \
    aws/ecs/qbot-task-definition.json > /tmp/qbot-task-def.json

# Register task definition
TASK_DEF_ARN=$(aws ecs register-task-definition \
  --cli-input-json file:///tmp/qbot-task-def.json \
  --query 'taskDefinition.taskDefinitionArn' --output text)
```

### Step 3: Deploy Service

```bash
aws cloudformation deploy \
  --template-file aws/cloudformation/service.yml \
  --stack-name qbot-service \
  --parameter-overrides \
    TaskDefinitionArn="$TASK_DEF_ARN" \
    StackName="qbot-infrastructure" \
  --region us-east-1
```

## ⚙️ Configuration

### Environment Variables

The following environment variables are used:

| Variable | Description | Default |
|----------|-------------|---------|
| `DISCORD_TOKEN` | Discord bot token (stored in Parameter Store) | Required |
| `RUST_LOG` | Logging level | `info` |
| `OLLAMA_URL` | Ollama service URL | `http://localhost:11434` |
| `OLLAMA_KEEP_ALIVE` | How long to keep models loaded | `1h` |

### Customizing Deployment

You can customize the deployment by modifying parameters:

```bash
# Deploy with custom EC2 parameters
export DISCORD_TOKEN="your_token_here"
export INSTANCE_TYPE="t3.large"        # Default: t3.medium
export KEY_PAIR_NAME="my-key-pair"      # Optional: for SSH access
export MIN_SIZE="1"                     # Default: 1
export MAX_SIZE="5"                     # Default: 3
export DESIRED_CAPACITY="2"             # Default: 1

./aws/scripts/deploy.sh

# Or manually deploy with CloudFormation
aws cloudformation deploy \
  --template-file aws/cloudformation/infrastructure.yml \
  --stack-name qbot-infrastructure \
  --parameter-overrides \
    DiscordToken="$DISCORD_TOKEN" \
    InstanceType="t3.large" \
    KeyPairName="my-key-pair" \
    MinSize="1" \
    MaxSize="5" \
    DesiredCapacity="2" \
    VpcCIDR="10.1.0.0/16" \
    ClusterName="my-qbot-cluster" \
  --capabilities CAPABILITY_NAMED_IAM
```

### EC2 Instance Types

Choose the appropriate instance type based on your needs:

| Instance Type | vCPU | Memory | Network | Use Case |
|---------------|------|---------|---------|----------|
| `t3.small` | 2 | 2 GB | Up to 5 Gbps | Development/Testing |
| `t3.medium` | 2 | 4 GB | Up to 5 Gbps | **Default - Light Production** |
| `t3.large` | 2 | 8 GB | Up to 5 Gbps | Medium load |
| `c5.large` | 2 | 4 GB | Up to 10 Gbps | CPU-intensive workloads |
| `c5.xlarge` | 4 | 8 GB | Up to 10 Gbps | High-performance needs |

### Resource Sizing

**Default Configuration (per EC2 instance):**
- Instance Type: t3.medium (2 vCPU, 4 GB RAM)
- Container CPU: 512 CPU units each (QBot + Ollama)
- Container Memory: 1 GB each (QBot + Ollama)
- EFS: 20 MB/s provisioned throughput

**To modify container resources**, edit `aws/ecs/qbot-task-definition.json`:

```json
{
  "containerDefinitions": [
    {
      "name": "ollama",
      "cpu": 1024,           // 1 vCPU for Ollama
      "memory": 2048,        // 2 GB for Ollama
      "memoryReservation": 1024
    },
    {
      "name": "qbot",
      "cpu": 512,            // 0.5 vCPU for QBot
      "memory": 1024,        // 1 GB for QBot
      "memoryReservation": 512
    }
  ]
}
```

## 📊 Monitoring & Troubleshooting

### CloudWatch Logs

View logs in the AWS Console or via CLI:

```bash
# View QBot logs
aws logs tail /ecs/qbot --follow

# View recent errors
aws logs filter-log-events \
  --log-group-name /ecs/qbot \
  --filter-pattern "ERROR"
```

### Common Issues

#### 1. Container Won't Start

**Symptoms**: Task keeps restarting
**Solution**: Check logs for startup errors:

```bash
aws logs describe-log-streams --log-group-name /ecs/qbot
aws logs get-log-events --log-group-name /ecs/qbot --log-stream-name <stream-name>
```

#### 2. Discord Token Issues

**Symptoms**: "missing DISCORD_TOKEN" in logs
**Solution**: Verify parameter store value:

```bash
aws ssm get-parameter --name /qbot/discord-token --with-decryption
```

#### 3. Ollama Model Download Issues

**Symptoms**: Long startup time, timeouts
**Solution**: Check EFS connectivity and increase health check grace period

#### 4. Out of Memory

**Symptoms**: Tasks being killed (OOMKilled)
**Solution**: Increase memory allocation in task definition

### Health Checks

The deployment includes health checks:
- **Ollama**: HTTP check on port 11434
- **ECS Service**: Built-in health monitoring

### Debugging Commands

```bash
# Get service status
aws ecs describe-services --cluster qbot-cluster --services qbot-service

# Get task details
aws ecs describe-tasks --cluster qbot-cluster --tasks <task-arn>

# Execute command in running container
aws ecs execute-command \
  --cluster qbot-cluster \
  --task <task-arn> \
  --container qbot \
  --interactive \
  --command "/bin/bash"
```

## 📈 Scaling & Management

### Manual Scaling

```bash
# Scale up to 2 instances
aws ecs update-service \
  --cluster qbot-cluster \
  --service qbot-service \
  --desired-count 2
```

### Auto Scaling

Enable auto scaling during deployment:

```bash
aws cloudformation deploy \
  --template-file aws/cloudformation/service.yml \
  --stack-name qbot-service \
  --parameter-overrides \
    EnableAutoScaling="true" \
    DesiredCount=1
```

Auto scaling triggers:
- **Scale Out**: CPU > 70% for 5 minutes
- **Scale In**: CPU < 30% for 5 minutes
- **Range**: 1-10 instances

### Updating the Application

To deploy a new version:

1. **Update the container image** in the task definition
2. **Register new task definition**:
   ```bash
   aws ecs register-task-definition --cli-input-json file://updated-task-def.json
   ```
3. **Update the service**:
   ```bash
   aws ecs update-service --cluster qbot-cluster --service qbot-service --task-definition qbot-task:2
   ```

### Rolling Updates

ECS automatically performs rolling updates with zero downtime:
- **Deployment strategy**: Rolling update
- **Maximum healthy**: 200%
- **Minimum healthy**: 100%

## 💰 Cost Estimation

### Monthly Costs (us-east-1, as of 2024)

**Minimal Setup (1 t3.medium instance):**
- EC2 t3.medium: ~$30/month (24/7)
- NAT Gateway: ~$32/month
- EFS Storage (1GB): ~$0.30/month
- Data Transfer: ~$5/month
- **Total: ~$67/month**

**High Availability (2 t3.medium instances, multi-AZ):**
- EC2 t3.medium (2 instances): ~$60/month
- NAT Gateway (2): ~$64/month
- EFS Storage: ~$0.30/month
- Data Transfer: ~$10/month
- **Total: ~$134/month**

**Cost by Instance Type (per instance, 24/7):**
| Instance Type | Monthly Cost | Use Case |
|---------------|-------------|----------|
| t3.small | ~$15 | Development |
| t3.medium | ~$30 | **Default** |
| t3.large | ~$60 | High load |
| c5.large | ~$62 | CPU intensive |

### Cost Optimization Tips

1. **Use Spot instances** for development (up to 70% savings):
   ```bash
   # Modify LaunchTemplate in infrastructure.yml
   InstanceMarketOptions:
     MarketType: spot
     SpotOptions:
       MaxPrice: "0.05"  # Max price per hour
   ```

2. **Schedule instances** for development:
   - Stop instances outside business hours
   - Use Lambda + EventBridge for automation

3. **Right-size instances**:
   - Start with t3.small for testing
   - Monitor CloudWatch metrics
   - Scale up only when needed

2. **Reduce NAT Gateway costs**:
   - Use VPC Endpoints for AWS services
   - Consider single NAT Gateway for non-critical workloads

3. **Right-size resources**:
   - Monitor CPU/memory usage
   - Adjust task definition accordingly

## 🔒 Security Considerations

### Network Security

- **Private subnets**: Containers run in private subnets with no direct internet access
- **NAT Gateway**: Controlled outbound internet access
- **Security Groups**: Restrictive ingress rules

### Secrets Management

- **Discord Token**: Stored encrypted in Parameter Store
- **IAM Roles**: Least privilege access
- **EFS Encryption**: Data encrypted at rest and in transit

### Best Practices

1. **Regular Updates**: Keep container images updated
2. **Access Logging**: Enable CloudTrail for API calls
3. **Resource Tagging**: Tag all resources for cost tracking
4. **Backup Strategy**: EFS automatically backs up data

### Security Groups Configuration

```yaml
# ECS Security Group - Only allows outbound traffic
ECSSecurityGroup:
  SecurityGroupEgress:
    - IpProtocol: -1
      CidrIp: 0.0.0.0/0  # Required for Discord API and Docker pulls

# EC2 Security Group - For ECS container instances
EC2SecurityGroup:
  SecurityGroupIngress:
    - IpProtocol: tcp
      FromPort: 22
      ToPort: 22
      CidrIp: 0.0.0.0/0  # SSH access (restrict to your IP for security)
    - IpProtocol: tcp
      FromPort: 32768
      ToPort: 65535
      SourceSecurityGroupId: !Ref ECSSecurityGroup  # Dynamic ports for containers
  SecurityGroupEgress:
    - IpProtocol: -1
      CidrIp: 0.0.0.0/0  # Required for Docker pulls, OS updates

# EFS Security Group - Only allows NFS from ECS and EC2
EFSSecurityGroup:
  SecurityGroupIngress:
    - IpProtocol: tcp
      FromPort: 2049
      ToPort: 2049
      SourceSecurityGroupId: !Ref ECSSecurityGroup
    - IpProtocol: tcp
      FromPort: 2049
      ToPort: 2049
      SourceSecurityGroupId: !Ref EC2SecurityGroup
```

**Security Notes:**
- SSH access is configured for all IPs (0.0.0.0/0) - **restrict this to your IP in production**
- EC2 instances communicate with containers via dynamic ports (32768-65535)
- EFS is only accessible from ECS tasks and EC2 instances in the security groups

## 🧹 Cleanup

To remove all AWS resources and stop incurring costs:

### Automated Cleanup

```bash
# Clean up everything
./aws/scripts/cleanup.sh

# Force cleanup without confirmation
./aws/scripts/cleanup.sh --force
```

### Manual Cleanup

```bash
# Delete service first
aws cloudformation delete-stack --stack-name qbot-service

# Wait for service deletion
aws cloudformation wait stack-delete-complete --stack-name qbot-service

# Delete infrastructure
aws cloudformation delete-stack --stack-name qbot-infrastructure

# Wait for infrastructure deletion
aws cloudformation wait stack-delete-complete --stack-name qbot-infrastructure
```

**⚠️ Warning**: Cleanup will permanently delete:
- All Ollama models stored in EFS
- Log data in CloudWatch
- Network infrastructure

## ❓ FAQ

### Q: Can I use an existing VPC?

**A**: Yes, modify the CloudFormation template to import existing VPC resources instead of creating new ones.

### Q: How do I add the bot to Discord servers?

**A**: Use the OAuth2 URL from Discord Developer Portal:
```
https://discord.com/oauth2/authorize?client_id=YOUR_BOT_CLIENT_ID&permissions=YOUR_PERMISSIONS&scope=bot
```

### Q: Can I run this in multiple regions?

**A**: Yes, deploy to each region separately. Each deployment is independent.

### Q: How do I backup Ollama models?

**A**: EFS automatically creates backups. For manual backup:
```bash
# Create EFS backup
aws efs create-backup-vault --backup-vault-name qbot-backup
```

### Q: What if I want to use a different LLM?

**A**: Modify the task definition to use a different container image or configure Ollama with different models.

### Q: How do I enable HTTPS/SSL?

**A**: QBot is a Discord bot and doesn't serve HTTP traffic. All communication is through Discord's API over HTTPS.

### Q: Can I use RDS instead of local storage?

**A**: QBot doesn't use a database currently. Ollama stores models on the filesystem, which is why we use EFS.

### Q: How do I monitor costs?

**A**: 
1. Set up billing alerts in AWS
2. Use AWS Cost Explorer
3. Tag resources for cost tracking
4. Monitor the `/ecs/qbot` log group size

### Q: What happens if Discord is down?

**A**: The bot will attempt to reconnect automatically. ECS will restart the container if it crashes.

### Q: Can I run multiple bots with this setup?

**A**: Yes, modify the service to run multiple tasks or deploy separate stacks for each bot.

## 🆘 Support

If you encounter issues:

1. **Check the logs**: CloudWatch Logs are your first debugging tool
2. **GitHub Issues**: Report bugs at https://github.com/ThatXliner/qbot/issues
3. **AWS Documentation**: Refer to AWS ECS and Fargate documentation
4. **Discord API**: Check Discord API status and documentation

---

**Happy Deploying! 🚀**

*This deployment guide ensures QBot runs reliably on AWS with proper monitoring, security, and scalability.*