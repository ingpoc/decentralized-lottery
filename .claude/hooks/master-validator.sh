#!/bin/bash

# Master Validator Hook
# Orchestrates all validation hooks in the correct order

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [MASTER-VALIDATOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [MASTER-VALIDATOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [MASTER-VALIDATOR]${NC} $1"
}

# Get the directory where this script is located
HOOKS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Main validation orchestration
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    
    log_info "Starting validation pipeline for $hook_type"
    [[ -n "$file_path" ]] && log_info "Target file: $(basename "$file_path")"
    
    case "$hook_type" in
        "pre-edit")
            # Run validations in priority order
            
            # 1. Version and environment consistency (critical)
            log_info "Running version validation..."
            if ! "$HOOKS_DIR/version-guard.sh"; then
                log_error "Version validation failed"
                exit 1
            fi
            
            # 2. IDE diagnostics check (prevents syntax errors)  
            log_info "Running IDE diagnostics check..."
            if ! "$HOOKS_DIR/ide-diagnostics-guard.sh"; then
                log_error "IDE diagnostics validation failed"
                exit 1
            fi
            
            # 3. Solana-specific patterns (prevents compilation errors)
            log_info "Running Solana pattern validation..."
            if ! "$HOOKS_DIR/solana-validator.sh"; then
                log_error "Solana pattern validation failed"
                exit 1
            fi
            
            # 4. MCP schema validation (ensures integration compatibility)
            log_info "Running MCP schema validation..."
            if ! "$HOOKS_DIR/mcp-schema-validator.sh"; then
                log_error "MCP schema validation failed"
                exit 1
            fi
            ;;
            
        "pre-bash")
            # For bash commands, focus on environment validation
            log_info "Running environment validation for bash command..."
            if ! "$HOOKS_DIR/version-guard.sh"; then
                log_error "Environment validation failed"
                exit 1
            fi
            
            if ! "$HOOKS_DIR/project-validator.sh"; then
                log_error "Project validation failed"
                exit 1
            fi
            ;;
            
        "session-start"|"project-start")
            # Full system validation at startup
            log_info "Running full system validation..."
            
            "$HOOKS_DIR/version-guard.sh" || exit 1
            "$HOOKS_DIR/project-validator.sh" || exit 1
            
            # Additional startup validations
            log_info "Checking project health..."
            ;;
            
        *)
            log_info "No specific validations for hook type: $hook_type"
            ;;
    esac
    
    log_success "All validations passed successfully"
}

# Error handling
trap 'log_error "Validation pipeline failed"; exit 1' ERR

# Run main function
main "$@"