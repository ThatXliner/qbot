#!/bin/bash

# QBot AWS Configuration Validator
# This script validates the AWS deployment configuration files

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

log() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

success() {
    echo -e "${GREEN}[SUCCESS]${NC} $1"
}

check_file_exists() {
    local file="$1"
    local description="$2"
    
    if [[ -f "$file" ]]; then
        success "$description exists: $file"
        return 0
    else
        error "$description not found: $file"
        return 1
    fi
}

validate_json() {
    local file="$1"
    local description="$2"
    
    if jq empty "$file" 2>/dev/null; then
        success "$description is valid JSON"
        return 0
    else
        error "$description contains invalid JSON"
        return 1
    fi
}

validate_yaml() {
    local file="$1"
    local description="$2"
    
    # For CloudFormation templates, we'll use cfn-lint if available, or basic syntax check
    if [[ "$file" == *"cloudformation"* ]]; then
        # CloudFormation YAML uses special tags, so we'll do a basic syntax check
        if python3 -c "
import yaml
import sys

# Custom constructor for CloudFormation tags
def multi_constructor(loader, tag_suffix, node):
    return None

# Add constructors for all CloudFormation intrinsic functions
yaml.SafeLoader.add_multi_constructor('!', multi_constructor)

try:
    with open('$file', 'r') as f:
        yaml.safe_load(f)
    print('valid')
except Exception as e:
    print(f'error: {e}')
    sys.exit(1)
" 2>/dev/null | grep -q "valid"; then
            success "$description is valid CloudFormation YAML"
            return 0
        else
            error "$description contains invalid YAML syntax"
            return 1
        fi
    else
        # Standard YAML validation
        if python3 -c "import yaml; yaml.safe_load(open('$file'))" 2>/dev/null; then
            success "$description is valid YAML"
            return 0
        else
            error "$description contains invalid YAML"
            return 1
        fi
    fi
}

validate_cloudformation() {
    local file="$1"
    local description="$2"
    
    if command -v aws &> /dev/null; then
        # Try to validate with AWS CLI - need to set region
        local region="${AWS_DEFAULT_REGION:-us-east-1}"
        if AWS_DEFAULT_REGION="$region" aws cloudformation validate-template --template-body "file://$file" &>/dev/null; then
            success "$description is valid CloudFormation template"
            return 0
        else
            warn "$description CloudFormation validation skipped (requires AWS credentials and region)"
            return 0
        fi
    else
        warn "AWS CLI not available, skipping CloudFormation validation for $description"
        return 0
    fi
}

check_task_definition_requirements() {
    local file="$1"
    
    log "Checking task definition requirements..."
    
    # Check required fields
    local required_fields=(
        ".family"
        ".networkMode"
        ".requiresCompatibilities"
        ".cpu"
        ".memory"
        ".containerDefinitions"
    )
    
    local missing_fields=()
    
    for field in "${required_fields[@]}"; do
        if ! jq -e "$field" "$file" &>/dev/null; then
            missing_fields+=("$field")
        fi
    done
    
    if [[ ${#missing_fields[@]} -eq 0 ]]; then
        success "All required task definition fields present"
    else
        error "Missing required fields: ${missing_fields[*]}"
        return 1
    fi
    
    # Check container definitions
    local container_count
    container_count=$(jq '.containerDefinitions | length' "$file")
    
    if [[ $container_count -eq 2 ]]; then
        success "Task definition has 2 containers (qbot + ollama)"
    else
        error "Task definition should have exactly 2 containers, found: $container_count"
        return 1
    fi
    
    # Check for required container names
    local qbot_container
    local ollama_container
    
    qbot_container=$(jq -r '.containerDefinitions[] | select(.name == "qbot") | .name' "$file")
    ollama_container=$(jq -r '.containerDefinitions[] | select(.name == "ollama") | .name' "$file")
    
    if [[ "$qbot_container" == "qbot" ]]; then
        success "QBot container definition found"
    else
        error "QBot container definition not found"
        return 1
    fi
    
    if [[ "$ollama_container" == "ollama" ]]; then
        success "Ollama container definition found"
    else
        error "Ollama container definition not found"
        return 1
    fi
}

check_cloudformation_parameters() {
    local file="$1"
    local description="$2"
    
    log "Checking CloudFormation parameters for $description..."
    
    if [[ "$file" == *"infrastructure.yml" ]]; then
        # Check for required parameters
        local required_params=("DiscordToken")
        
        for param in "${required_params[@]}"; do
            if grep -q "$param:" "$file"; then
                success "Required parameter '$param' found"
            else
                error "Required parameter '$param' not found"
                return 1
            fi
        done
    fi
}

validate_script_permissions() {
    local script="$1"
    local description="$2"
    
    if [[ -x "$script" ]]; then
        success "$description is executable"
        return 0
    else
        error "$description is not executable"
        warn "Run: chmod +x $script"
        return 1
    fi
}

main() {
    log "QBot AWS Configuration Validator"
    echo
    
    local validation_errors=0
    
    # Check directory structure
    log "Validating directory structure..."
    
    local dirs=(
        "aws"
        "aws/ecs"
        "aws/cloudformation"
        "aws/scripts"
    )
    
    for dir in "${dirs[@]}"; do
        if [[ -d "$dir" ]]; then
            success "Directory exists: $dir"
        else
            error "Directory missing: $dir"
            ((validation_errors++))
        fi
    done
    
    echo
    
    # Validate files exist
    log "Checking file existence..."
    
    local files=(
        "aws/ecs/qbot-task-definition.json:Task Definition"
        "aws/cloudformation/infrastructure.yml:Infrastructure Template"
        "aws/cloudformation/service.yml:Service Template"
        "aws/scripts/deploy.sh:Deploy Script"
        "aws/scripts/cleanup.sh:Cleanup Script"
        "DEPLOY.md:Deployment Documentation"
    )
    
    for file_desc in "${files[@]}"; do
        local file="${file_desc%:*}"
        local desc="${file_desc#*:}"
        
        if ! check_file_exists "$file" "$desc"; then
            ((validation_errors++))
        fi
    done
    
    echo
    
    # Validate JSON files
    log "Validating JSON files..."
    
    if [[ -f "aws/ecs/qbot-task-definition.json" ]]; then
        if validate_json "aws/ecs/qbot-task-definition.json" "Task Definition"; then
            check_task_definition_requirements "aws/ecs/qbot-task-definition.json" || ((validation_errors++))
        else
            ((validation_errors++))
        fi
    fi
    
    echo
    
    # Validate YAML files
    log "Validating YAML files..."
    
    local yaml_files=(
        "aws/cloudformation/infrastructure.yml:Infrastructure Template"
        "aws/cloudformation/service.yml:Service Template"
    )
    
    for file_desc in "${yaml_files[@]}"; do
        local file="${file_desc%:*}"
        local desc="${file_desc#*:}"
        
        if [[ -f "$file" ]]; then
            if validate_yaml "$file" "$desc"; then
                validate_cloudformation "$file" "$desc" || ((validation_errors++))
                check_cloudformation_parameters "$file" "$desc" || ((validation_errors++))
            else
                ((validation_errors++))
            fi
        fi
    done
    
    echo
    
    # Validate script permissions
    log "Validating script permissions..."
    
    local scripts=(
        "aws/scripts/deploy.sh:Deploy Script"
        "aws/scripts/cleanup.sh:Cleanup Script"
    )
    
    for script_desc in "${scripts[@]}"; do
        local script="${script_desc%:*}"
        local desc="${script_desc#*:}"
        
        if [[ -f "$script" ]]; then
            validate_script_permissions "$script" "$desc" || ((validation_errors++))
        fi
    done
    
    echo
    
    # Summary
    if [[ $validation_errors -eq 0 ]]; then
        success "✅ All validations passed! Configuration is ready for deployment."
    else
        error "❌ Found $validation_errors validation error(s). Please fix them before deploying."
        exit 1
    fi
}

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --help)
            echo "QBot AWS Configuration Validator"
            echo
            echo "Usage: $0"
            echo
            echo "This script validates all AWS deployment configuration files:"
            echo "- Directory structure"
            echo "- JSON syntax (task definitions)"
            echo "- YAML syntax (CloudFormation templates)"
            echo "- CloudFormation template validation"
            echo "- Script permissions"
            echo "- Required parameters and fields"
            exit 0
            ;;
        *)
            error "Unknown option: $1. Use --help for usage information."
            exit 1
            ;;
    esac
done

main