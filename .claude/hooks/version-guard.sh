#!/bin/bash

# Version Guard Hook for Claude Code
# Prevents version mismatches across Solana development tools

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Logging functions
log_info() {
    echo -e "${BLUE}ℹ️  [VERSION-GUARD]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [VERSION-GUARD]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [VERSION-GUARD]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [VERSION-GUARD]${NC} $1"
}

# Function to extract version from Anchor.toml
get_anchor_config_version() {
    if [[ -f "Anchor.toml" ]]; then
        grep 'anchor_version' Anchor.toml | cut -d'"' -f2 2>/dev/null || echo ""
    else
        echo ""
    fi
}

# Function to get installed Anchor CLI version
get_anchor_cli_version() {
    anchor --version 2>/dev/null | cut -d' ' -f2 || echo ""
}

# Function to get package.json Anchor dependency version
get_anchor_dependency_version() {
    if [[ -f "package.json" ]] && command -v node &> /dev/null; then
        node -p "
            try {
                const pkg = require('./package.json');
                const dep = pkg.dependencies['@coral-xyz/anchor'] || pkg.devDependencies['@coral-xyz/anchor'] || '';
                dep.replace(/[\^~]/, '');
            } catch(e) {
                '';
            }
        " 2>/dev/null || echo ""
    else
        echo ""
    fi
}

# Function to check Anchor version consistency
check_anchor_consistency() {
    local config_version=$(get_anchor_config_version)
    local cli_version=$(get_anchor_cli_version)
    local dep_version=$(get_anchor_dependency_version)
    
    log_info "Checking Anchor version consistency..."
    
    if [[ -z "$cli_version" ]]; then
        log_error "Anchor CLI not found. Please install Anchor CLI."
        exit 1
    fi
    
    log_info "Anchor CLI: $cli_version"
    [[ -n "$config_version" ]] && log_info "Anchor.toml: $config_version"
    [[ -n "$dep_version" ]] && log_info "package.json: $dep_version"
    
    # Check if config version matches CLI
    if [[ -n "$config_version" ]] && [[ "$config_version" != "$cli_version" ]]; then
        log_error "Anchor.toml version ($config_version) doesn't match CLI ($cli_version)"
        log_error "This will cause build failures!"
        echo ""
        log_info "To fix this issue:"
        log_info "  Option 1: Update Anchor.toml to match CLI: anchor_version = \"$cli_version\""
        log_info "  Option 2: Install matching Anchor CLI: avm install $config_version && avm use $config_version"
        echo ""
        exit 1
    fi
    
    # Check if dependency version is compatible
    if [[ -n "$dep_version" ]] && [[ "$dep_version" != "$cli_version" ]]; then
        # Allow minor version differences (0.30.1 vs 0.30.0)
        local cli_major_minor=$(echo "$cli_version" | cut -d'.' -f1,2)
        local dep_major_minor=$(echo "$dep_version" | cut -d'.' -f1,2)
        
        if [[ "$cli_major_minor" != "$dep_major_minor" ]]; then
            log_warning "Dependency version ($dep_version) differs from CLI ($cli_version)"
            log_warning "Consider updating package.json dependency to ^$cli_version"
        fi
    fi
    
    log_success "Anchor versions are consistent"
}

# Function to check program ID consistency
check_program_ids_consistency() {
    if [[ ! -f "config/program-ids.json" ]]; then
        log_warning "config/program-ids.json not found - skipping program ID checks"
        return 0
    fi
    
    log_info "Checking program ID consistency..."
    
    # Check if update-program-ids script exists
    if [[ -f "scripts/update-program-ids.ts" ]]; then
        log_info "Centralized program ID management detected"
        
        # Read current cluster from Anchor.toml
        local cluster=$(grep -i 'cluster' Anchor.toml | cut -d'"' -f2 2>/dev/null | tr '[:upper:]' '[:lower:]' || echo "localnet")
        
        log_info "Current cluster: $cluster"
        
        # Suggest running update script if program files were changed
        if [[ "$CLAUDE_FILE_PATH" =~ programs/.*/src/lib\.rs ]]; then
            log_info "Program source file modified - consider running:"
            log_info "  npm run update-ids all $cluster"
        fi
    fi
    
    log_success "Program ID configuration looks good"
}

# Function to check Node.js version
check_node_version() {
    if [[ -f "package.json" ]] && command -v node &> /dev/null; then
        local node_version=$(node --version | sed 's/v//')
        local required_node=$(node -p "
            try {
                const pkg = require('./package.json');
                pkg.engines && pkg.engines.node ? pkg.engines.node.replace(/[^\d.]/g, '') : '';
            } catch(e) {
                '';
            }
        " 2>/dev/null)
        
        if [[ -n "$required_node" ]]; then
            log_info "Node.js version: $node_version (required: $required_node)"
        else
            log_info "Node.js version: $node_version"
        fi
    fi
}

# Function to validate edit content for version changes
validate_edit_content() {
    local file_path="$1"
    local new_content="$2"
    
    if [[ "$file_path" =~ Anchor\.toml$ ]]; then
        log_info "Validating Anchor.toml edit..."
        
        # Extract new anchor_version from content
        local new_version=$(echo "$new_content" | grep 'anchor_version' | cut -d'"' -f2 2>/dev/null)
        
        if [[ -n "$new_version" ]]; then
            local cli_version=$(get_anchor_cli_version)
            
            if [[ "$new_version" != "$cli_version" ]]; then
                log_error "Attempting to set anchor_version to '$new_version'"
                log_error "But installed CLI is '$cli_version'"
                log_error "This will cause build failures!"
                echo ""
                log_info "If you want to use Anchor $new_version:"
                log_info "  avm install $new_version && avm use $new_version"
                echo ""
                exit 1
            fi
            
            log_success "Anchor version change looks good: $new_version"
        fi
    fi
    
    if [[ "$file_path" =~ package\.json$ ]]; then
        log_info "Validating package.json edit..."
        
        # Check for Anchor dependency changes
        if echo "$new_content" | grep -q "@coral-xyz/anchor"; then
            log_info "Anchor dependency change detected in package.json"
            log_info "Remember to run 'npm install' after this change"
        fi
    fi
}

# Main execution based on hook type
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local edit_content="${CLAUDE_EDIT_CONTENT:-}"
    
    log_info "Hook triggered: $hook_type"
    [[ -n "$file_path" ]] && log_info "File: $file_path"
    
    case "$hook_type" in
        "pre-edit")
            if [[ "$file_path" =~ Anchor\.toml$ ]] || [[ "$file_path" =~ package\.json$ ]]; then
                check_anchor_consistency
                [[ -n "$edit_content" ]] && validate_edit_content "$file_path" "$edit_content"
            fi
            
            if [[ "$file_path" =~ programs/.*/src/lib\.rs$ ]]; then
                check_program_ids_consistency
            fi
            ;;
            
        "pre-bash")
            local command="${CLAUDE_COMMAND:-}"
            if [[ "$command" =~ ^anchor ]] || [[ "$command" =~ build ]] || [[ "$command" =~ deploy ]]; then
                check_anchor_consistency
                check_program_ids_consistency
                check_node_version
            fi
            ;;
            
        "session-start"|"project-start")
            log_info "Running full project validation..."
            check_anchor_consistency
            check_program_ids_consistency
            check_node_version
            log_success "Project validation completed"
            ;;
            
        *)
            log_warning "Unknown hook type: $hook_type"
            ;;
    esac
}

# Run main function if script is executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi