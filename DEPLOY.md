# 🚀 QBot AWS Deployment Guide

This guide provides detailed instructions for deploying QBot Discord bot to AWS using Amazon ECS (Elastic Container Service) with Fargate.

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
│  │  │   NAT Gateway   │    │  │       ECS Fargate       │ │ │ │
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

- **ECS Fargate**: Serverless container execution
- **VPC**: Isolated network with public/private subnets
- **EFS**: Persistent storage for Ollama models
- **Systems Manager**: Secure storage for Discord token
- **CloudWatch**: Logging and monitoring
- **NAT Gateway**: Outbound internet access for containers

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
# Deploy with custom parameters
aws cloudformation deploy \
  --template-file aws/cloudformation/infrastructure.yml \
  --stack-name qbot-infrastructure \
  --parameter-overrides \
    DiscordToken="$DISCORD_TOKEN" \
    VpcCIDR="10.1.0.0/16" \
    ClusterName="my-qbot-cluster" \
  --capabilities CAPABILITY_NAMED_IAM
```

### Resource Sizing

**Default Configuration:**
- CPU: 1 vCPU (0.5 for each container)
- Memory: 2 GB (1 GB for each container)
- EFS: 20 MB/s provisioned throughput

**To modify resources**, edit `aws/ecs/qbot-task-definition.json`:

```json
{
  "cpu": "2048",     // 2 vCPU
  "memory": "4096",  // 4 GB
  "containerDefinitions": [
    {
      "name": "qbot",
      "cpu": 1024,        // 1 vCPU for QBot
      "memory": 2048      // 2 GB for QBot
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

**Minimal Setup (1 instance):**
- ECS Fargate (1 vCPU, 2GB): ~$30/month
- NAT Gateway: ~$32/month
- EFS Storage (1GB): ~$0.30/month
- Data Transfer: ~$5/month
- **Total: ~$67/month**

**High Availability (2 AZ, 2 instances):**
- ECS Fargate (2 instances): ~$60/month
- NAT Gateway (2): ~$64/month
- EFS Storage: ~$0.30/month
- ALB (if added): ~$16/month
- Data Transfer: ~$10/month
- **Total: ~$150/month**

### Cost Optimization Tips

1. **Use Spot instances** for development:
   ```yaml
   # Add to service.yml
   CapacityProviderStrategy:
     - CapacityProvider: FARGATE_SPOT
       Weight: 100
   ```

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

# EFS Security Group - Only allows NFS from ECS
EFSSecurityGroup:
  SecurityGroupIngress:
    - IpProtocol: tcp
      FromPort: 2049
      ToPort: 2049
      SourceSecurityGroupId: !Ref ECSSecurityGroup
```

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