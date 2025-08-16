# QBot AWS Deployment Files

This directory contains all the necessary files for deploying QBot to AWS using ECS Fargate.

## 📁 Directory Structure

```
aws/
├── cloudformation/
│   ├── infrastructure.yml    # VPC, EFS, IAM, and core infrastructure
│   └── service.yml          # ECS service definition
├── ecs/
│   └── qbot-task-definition.json  # Container task definition
└── scripts/
    ├── deploy.sh           # Automated deployment script
    ├── cleanup.sh          # Cleanup/deletion script
    └── validate.sh         # Configuration validation script
```

## 🚀 Quick Deployment

```bash
# Validate configuration
./aws/scripts/validate.sh

# Deploy to AWS
export DISCORD_TOKEN="your_token_here"
./aws/scripts/deploy.sh
```

## 📚 Documentation

See the main [DEPLOY.md](../DEPLOY.md) file for comprehensive deployment instructions, troubleshooting, and configuration options.

## 🔧 Customization

To customize the deployment:

1. **Resource Sizing**: Edit `ecs/qbot-task-definition.json` to adjust CPU/memory
2. **Networking**: Modify `cloudformation/infrastructure.yml` for custom VPC settings
3. **Scaling**: Update `cloudformation/service.yml` for auto-scaling configuration

## 🏗️ Architecture

The deployment creates:
- VPC with public/private subnets across 2 AZs
- ECS Fargate cluster running QBot and Ollama containers
- EFS file system for persistent Ollama model storage
- Systems Manager Parameter Store for secure Discord token storage
- CloudWatch Logs for monitoring

## 💰 Estimated Cost

- **Basic deployment**: ~$67/month
- **High availability**: ~$150/month

See [DEPLOY.md](../DEPLOY.md#cost-estimation) for detailed cost breakdown.

## 🆘 Troubleshooting

1. **Validate configuration**: `./aws/scripts/validate.sh`
2. **Check logs**: AWS Console → CloudWatch → Log Groups → `/ecs/qbot`
3. **Service status**: AWS Console → ECS → Clusters → `qbot-cluster`

## 🧹 Cleanup

To remove all AWS resources:

```bash
./aws/scripts/cleanup.sh
```

**⚠️ Warning**: This permanently deletes all data and cannot be undone.