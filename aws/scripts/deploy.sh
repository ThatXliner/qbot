#!/bin/bash

# QBot AWS Deployment Script
# This script deploys QBot to AWS using CloudFormation and ECS

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
STACK_NAME_INFRA="qbot-infrastructure"
STACK_NAME_SERVICE="qbot-service"
AWS_REGION="${AWS_REGION:-us-east-1}"
DISCORD_TOKEN="${DISCORD_TOKEN:-}"
INSTANCE_TYPE="${INSTANCE_TYPE:-t3.medium}"
KEY_PAIR_NAME="${KEY_PAIR_NAME:-}"
MIN_SIZE="${MIN_SIZE:-1}"
MAX_SIZE="${MAX_SIZE:-3}"
DESIRED_CAPACITY="${DESIRED_CAPACITY:-1}"

# Functions
log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
    exit 1
}

check_dependencies() {
    log "Checking dependencies..."
    
    if ! command -v aws &> /dev/null; then
        error "AWS CLI is not installed. Please install it first."
    fi
    
    if ! command -v jq &> /dev/null; then
        error "jq is not installed. Please install it first."
    fi
    
    # Check AWS credentials
    if ! aws sts get-caller-identity &> /dev/null; then
        error "AWS credentials not configured. Run 'aws configure' first."
    fi
    
    log "Dependencies check passed ✓"
}

validate_parameters() {
    log "Validating parameters..."
    
    if [[ -z "$DISCORD_TOKEN" ]]; then
        error "DISCORD_TOKEN environment variable is required"
    fi
    
    log "Parameters validation passed ✓"
}

deploy_infrastructure() {
    log "Deploying infrastructure stack..."
    
    aws cloudformation deploy \
        --template-file aws/cloudformation/infrastructure.yml \
        --stack-name "$STACK_NAME_INFRA" \
        --parameter-overrides \
            DiscordToken="$DISCORD_TOKEN" \
            ClusterName="qbot-cluster" \
            InstanceType="$INSTANCE_TYPE" \
            KeyPairName="$KEY_PAIR_NAME" \
            MinSize="$MIN_SIZE" \
            MaxSize="$MAX_SIZE" \
            DesiredCapacity="$DESIRED_CAPACITY" \
        --capabilities CAPABILITY_NAMED_IAM \
        --region "$AWS_REGION" \
        --tags \
            Project=QBot \
            Environment=Production
    
    if [[ $? -eq 0 ]]; then
        log "Infrastructure stack deployed successfully ✓"
    else
        error "Failed to deploy infrastructure stack"
    fi
}

register_task_definition() {
    log "Registering ECS task definition..."
    
    # Get outputs from infrastructure stack
    local account_id
    local efs_file_system_id
    local efs_access_point_id
    local execution_role_arn
    local task_role_arn
    
    account_id=$(aws sts get-caller-identity --query Account --output text)
    
    efs_file_system_id=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME_INFRA" \
        --region "$AWS_REGION" \
        --query 'Stacks[0].Outputs[?OutputKey==`EFSFileSystem`].OutputValue' \
        --output text)
    
    efs_access_point_id=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME_INFRA" \
        --region "$AWS_REGION" \
        --query 'Stacks[0].Outputs[?OutputKey==`EFSAccessPoint`].OutputValue' \
        --output text)
    
    execution_role_arn=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME_INFRA" \
        --region "$AWS_REGION" \
        --query 'Stacks[0].Outputs[?OutputKey==`ECSTaskExecutionRole`].OutputValue' \
        --output text)
    
    task_role_arn=$(aws cloudformation describe-stacks \
        --stack-name "$STACK_NAME_INFRA" \
        --region "$AWS_REGION" \
        --query 'Stacks[0].Outputs[?OutputKey==`ECSTaskRole`].OutputValue' \
        --output text)
    
    # Create temporary task definition with substituted values
    local temp_task_def="/tmp/qbot-task-definition-$(date +%s).json"
    
    sed -e "s/{AWS_ACCOUNT_ID}/$account_id/g" \
        -e "s/{AWS_REGION}/$AWS_REGION/g" \
        -e "s/{EFS_FILE_SYSTEM_ID}/$efs_file_system_id/g" \
        -e "s/{EFS_ACCESS_POINT_ID}/$efs_access_point_id/g" \
        aws/ecs/qbot-task-definition.json > "$temp_task_def"
    
    # Update the ARNs in the task definition
    jq --arg exec_role "$execution_role_arn" \
       --arg task_role "$task_role_arn" \
       '.executionRoleArn = $exec_role | .taskRoleArn = $task_role' \
       "$temp_task_def" > "${temp_task_def}.updated"
    
    mv "${temp_task_def}.updated" "$temp_task_def"
    
    # Register the task definition
    local task_def_arn
    task_def_arn=$(aws ecs register-task-definition \
        --cli-input-json "file://$temp_task_def" \
        --region "$AWS_REGION" \
        --query 'taskDefinition.taskDefinitionArn' \
        --output text)
    
    # Clean up temporary file
    rm "$temp_task_def"
    
    if [[ $? -eq 0 ]]; then
        log "Task definition registered successfully ✓"
        log "Task Definition ARN: $task_def_arn"
        echo "$task_def_arn"
    else
        error "Failed to register task definition"
    fi
}

deploy_service() {
    local task_def_arn="$1"
    
    log "Deploying ECS service..."
    
    aws cloudformation deploy \
        --template-file aws/cloudformation/service.yml \
        --stack-name "$STACK_NAME_SERVICE" \
        --parameter-overrides \
            StackName="$STACK_NAME_INFRA" \
            TaskDefinitionArn="$task_def_arn" \
            DesiredCount=1 \
        --capabilities CAPABILITY_IAM \
        --region "$AWS_REGION" \
        --tags \
            Project=QBot \
            Environment=Production
    
    if [[ $? -eq 0 ]]; then
        log "Service stack deployed successfully ✓"
    else
        error "Failed to deploy service stack"
    fi
}

wait_for_service() {
    log "Waiting for service to become stable..."
    
    local cluster_name="qbot-cluster"
    local service_name="qbot-service"
    
    aws ecs wait services-stable \
        --cluster "$cluster_name" \
        --services "$service_name" \
        --region "$AWS_REGION"
    
    if [[ $? -eq 0 ]]; then
        log "Service is now stable ✓"
    else
        warn "Service stability check timed out, but deployment may still be in progress"
    fi
}

show_status() {
    log "Deployment Status:"
    echo
    
    # Get cluster info
    local cluster_name="qbot-cluster"
    local service_name="qbot-service"
    
    # Service status
    local service_info
    service_info=$(aws ecs describe-services \
        --cluster "$cluster_name" \
        --services "$service_name" \
        --region "$AWS_REGION" \
        --query 'services[0]' 2>/dev/null)
    
    if [[ -n "$service_info" ]]; then
        local running_count
        local desired_count
        local status
        
        running_count=$(echo "$service_info" | jq -r '.runningCount')
        desired_count=$(echo "$service_info" | jq -r '.desiredCount')
        status=$(echo "$service_info" | jq -r '.status')
        
        echo -e "${BLUE}Service:${NC} $service_name"
        echo -e "${BLUE}Status:${NC} $status"
        echo -e "${BLUE}Running/Desired:${NC} $running_count/$desired_count"
        echo
        
        # Get recent events
        echo -e "${BLUE}Recent Events:${NC}"
        echo "$service_info" | jq -r '.events[:3][] | "\(.createdAt) - \(.message)"'
    else
        warn "Could not retrieve service status"
    fi
    
    echo
    log "You can monitor the deployment in the AWS Console:"
    echo -e "${BLUE}ECS Console:${NC} https://$AWS_REGION.console.aws.amazon.com/ecs/home?region=$AWS_REGION#/clusters/$cluster_name/services"
    echo -e "${BLUE}CloudWatch Logs:${NC} https://$AWS_REGION.console.aws.amazon.com/cloudwatch/home?region=$AWS_REGION#logsV2:log-groups/log-group/%2Fecs%2Fqbot"
}

cleanup_on_error() {
    if [[ $? -ne 0 ]]; then
        error "Deployment failed! Check the AWS Console for more details."
    fi
}

# Main deployment flow
main() {
    log "Starting QBot AWS deployment..."
    log "Region: $AWS_REGION"
    log "Infrastructure Stack: $STACK_NAME_INFRA"
    log "Service Stack: $STACK_NAME_SERVICE"
    echo
    
    trap cleanup_on_error EXIT
    
    check_dependencies
    validate_parameters
    
    deploy_infrastructure
    task_def_arn=$(register_task_definition)
    deploy_service "$task_def_arn"
    wait_for_service
    
    trap - EXIT  # Remove error trap
    
    log "✅ Deployment completed successfully!"
    echo
    show_status
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --discord-token)
            DISCORD_TOKEN="$2"
            shift 2
            ;;
        --help)
            echo "QBot AWS Deployment Script"
            echo
            echo "Usage: $0 [options]"
            echo
            echo "Options:"
            echo "  --region REGION          AWS region (default: us-east-1)"
            echo "  --discord-token TOKEN    Discord bot token"
            echo "  --help                   Show this help message"
            echo
            echo "Environment variables:"
            echo "  DISCORD_TOKEN           Discord bot token (required)"
            echo "  AWS_REGION             AWS region (default: us-east-1)"
            exit 0
            ;;
        *)
            error "Unknown option: $1. Use --help for usage information."
            ;;
    esac
done

# Run main function
main