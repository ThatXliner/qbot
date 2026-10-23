#!/bin/bash

# QBot AWS Cleanup Script
# This script removes all AWS resources created for QBot

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Configuration
STACK_NAME_INFRA="qbot-infrastructure"
STACK_NAME_SERVICE="qbot-service"
AWS_REGION="${AWS_REGION:-us-east-1}"

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

confirm_cleanup() {
    echo -e "${YELLOW}⚠️  WARNING: This will delete ALL QBot resources in AWS!${NC}"
    echo
    echo "This includes:"
    echo "- ECS Service and Tasks"
    echo "- EFS File System (and all stored Ollama models)"
    echo "- VPC and networking resources"
    echo "- IAM roles"
    echo "- Systems Manager parameters"
    echo
    echo -e "${RED}THIS ACTION CANNOT BE UNDONE!${NC}"
    echo
    read -p "Are you sure you want to continue? (type 'yes' to confirm): " -r
    echo
    
    if [[ ! $REPLY =~ ^yes$ ]]; then
        log "Cleanup cancelled by user."
        exit 0
    fi
}

cleanup_service() {
    log "Deleting service stack..."
    
    if aws cloudformation describe-stacks --stack-name "$STACK_NAME_SERVICE" --region "$AWS_REGION" &>/dev/null; then
        aws cloudformation delete-stack \
            --stack-name "$STACK_NAME_SERVICE" \
            --region "$AWS_REGION"
        
        log "Waiting for service stack deletion to complete..."
        aws cloudformation wait stack-delete-complete \
            --stack-name "$STACK_NAME_SERVICE" \
            --region "$AWS_REGION"
        
        log "Service stack deleted successfully ✓"
    else
        warn "Service stack does not exist, skipping..."
    fi
}

cleanup_infrastructure() {
    log "Deleting infrastructure stack..."
    
    if aws cloudformation describe-stacks --stack-name "$STACK_NAME_INFRA" --region "$AWS_REGION" &>/dev/null; then
        aws cloudformation delete-stack \
            --stack-name "$STACK_NAME_INFRA" \
            --region "$AWS_REGION"
        
        log "Waiting for infrastructure stack deletion to complete..."
        log "This may take several minutes as it needs to delete NAT Gateways, EFS mounts, etc."
        
        aws cloudformation wait stack-delete-complete \
            --stack-name "$STACK_NAME_INFRA" \
            --region "$AWS_REGION"
        
        log "Infrastructure stack deleted successfully ✓"
    else
        warn "Infrastructure stack does not exist, skipping..."
    fi
}

cleanup_task_definitions() {
    log "Cleaning up task definitions..."
    
    # List all QBot task definitions
    local task_defs
    task_defs=$(aws ecs list-task-definitions \
        --family-prefix qbot-task \
        --region "$AWS_REGION" \
        --query 'taskDefinitionArns' \
        --output text)
    
    if [[ -n "$task_defs" && "$task_defs" != "None" ]]; then
        for task_def in $task_defs; do
            log "Deregistering task definition: $task_def"
            aws ecs deregister-task-definition \
                --task-definition "$task_def" \
                --region "$AWS_REGION" \
                &>/dev/null || warn "Failed to deregister $task_def"
        done
        log "Task definitions cleaned up ✓"
    else
        warn "No task definitions found, skipping..."
    fi
}

main() {
    log "QBot AWS Cleanup"
    log "Region: $AWS_REGION"
    echo
    
    if [[ "$1" != "--force" ]]; then
        confirm_cleanup
    else
        warn "Force cleanup mode enabled, skipping confirmation"
    fi
    
    log "Starting cleanup process..."
    
    cleanup_service
    cleanup_infrastructure
    cleanup_task_definitions
    
    log "✅ Cleanup completed successfully!"
    log "All QBot resources have been removed from AWS."
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --region)
            AWS_REGION="$2"
            shift 2
            ;;
        --force)
            FORCE=true
            shift
            ;;
        --help)
            echo "QBot AWS Cleanup Script"
            echo
            echo "Usage: $0 [options]"
            echo
            echo "Options:"
            echo "  --region REGION    AWS region (default: us-east-1)"
            echo "  --force           Skip confirmation prompt"
            echo "  --help            Show this help message"
            exit 0
            ;;
        *)
            error "Unknown option: $1. Use --help for usage information."
            ;;
    esac
done

main "$@"