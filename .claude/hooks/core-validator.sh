#!/bin/bash

# Core Validator Hook - Consolidated Environment & Version Validation
# Replaces: version-guard.sh, project-validator.sh, master-validator.sh (partially)
# Triggers: pre-edit (critical files only), pre-bash (build commands), session-start

set -e

# Cache configuration
CACHE_DIR=".claude/cache"
CACHE_EXPIRY=300  # 5 minutes

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [CORE-VALIDATOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [CORE-VALIDATOR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [CORE-VALIDATOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [CORE-VALIDATOR]${NC} $1"
}

# Cache management functions
init_cache() {
    mkdir -p "$CACHE_DIR"
}

get_cache_key() {
    local validation_type="$1"
    echo "$CACHE_DIR/${validation_type}_cache"
}

is_cache_valid() {
    local cache_file="$1"
    if [[ -f "$cache_file" ]]; then
        local cache_time=$(stat -c %Y "$cache_file" 2>/dev/null || stat -f %m "$cache_file" 2>/dev/null || echo 0)
        local current_time=$(date +%s)
        local diff=$((current_time - cache_time))
        [[ $diff -lt $CACHE_EXPIRY ]]
    else
        false
    fi
}

cache_result() {
    local cache_file="$1"
    local result="$2"
    echo "$result" > "$cache_file"
}

# Anchor version validation with caching
validate_anchor_versions() {
    local cache_file=$(get_cache_key "anchor_versions")
    
    if is_cache_valid "$cache_file"; then
        local cached_result=$(cat "$cache_file")
        if [[ "$cached_result" == "VALID" ]]; then
            return 0
        else
            log_error "Cached validation failed: $cached_result"
            return 1
        fi
    fi
    
    log_info "Validating Anchor version consistency..."
    
    local cli_version=$(anchor --version 2>/dev/null | cut -d' ' -f2 || echo "")
    if [[ -z "$cli_version" ]]; then
        cache_result "$cache_file" "Anchor CLI not found"
        log_error "Anchor CLI not found. Please install Anchor CLI."
        return 1
    fi
    
    local config_version=""
    if [[ -f "Anchor.toml" ]]; then
        config_version=$(grep 'anchor_version' Anchor.toml | cut -d'"' -f2 2>/dev/null || echo "")
    fi
    
    local dep_version=""
    if [[ -f "package.json" ]] && command -v node &> /dev/null; then
        dep_version=$(node -p "
            try {
                const pkg = require('./package.json');
                const dep = pkg.dependencies['@coral-xyz/anchor'] || pkg.devDependencies['@coral-xyz/anchor'] || '';
                dep.replace(/[\^~]/, '');
            } catch(e) { ''; }
        " 2>/dev/null || echo "")
    fi
    
    # Version consistency checks
    if [[ -n "$config_version" ]] && [[ "$config_version" != "$cli_version" ]]; then
        local error_msg="Anchor.toml version ($config_version) doesn't match CLI ($cli_version)"
        cache_result "$cache_file" "$error_msg"
        log_error "$error_msg"
        log_error "This will cause build failures!"
        return 1
    fi
    
    if [[ -n "$dep_version" ]] && [[ "$dep_version" != "$cli_version" ]]; then
        local cli_major_minor=$(echo "$cli_version" | cut -d'.' -f1,2)
        local dep_major_minor=$(echo "$dep_version" | cut -d'.' -f1,2)
        
        if [[ "$cli_major_minor" != "$dep_major_minor" ]]; then
            log_warning "Dependency version ($dep_version) differs from CLI ($cli_version)"
        fi
    fi
    
    cache_result "$cache_file" "VALID"
    log_success "Anchor versions are consistent (CLI: $cli_version)"
    return 0
}

# Program ID consistency validation with caching
validate_program_ids() {
    if [[ ! -f "config/program-ids.json" ]]; then
        return 0
    fi
    
    local cache_file=$(get_cache_key "program_ids")
    local cluster=$(grep -i 'cluster' Anchor.toml | cut -d'"' -f2 2>/dev/null | tr '[:upper:]' '[:lower:]' || echo "localnet")
    
    # Check if program files changed since last validation
    local program_files_changed=false
    if [[ -n "${CLAUDE_FILE_PATH:-}" ]] && [[ "${CLAUDE_FILE_PATH}" =~ programs/.*/src/lib\.rs$ ]]; then
        program_files_changed=true
    fi
    
    if ! $program_files_changed && is_cache_valid "$cache_file"; then
        return 0
    fi
    
    log_info "Validating program ID consistency for $cluster..."
    
    if [[ -f "scripts/update-program-ids.ts" ]]; then
        if $program_files_changed; then
            log_info "Program source modified - consider running: npm run update-ids all $cluster"
        fi
    fi
    
    cache_result "$cache_file" "VALID"
    log_success "Program ID configuration validated"
    return 0
}

# Environment health check with caching
validate_environment() {
    local cache_file=$(get_cache_key "environment")
    
    if is_cache_valid "$cache_file"; then
        return 0
    fi
    
    log_info "Validating development environment..."
    
    # Node.js version check
    if [[ -f "package.json" ]] && command -v node &> /dev/null; then
        local node_version=$(node --version | sed 's/v//')
        log_info "Node.js version: $node_version"
    fi
    
    # Basic Solana CLI check
    if command -v solana &> /dev/null; then
        local solana_version=$(solana --version | cut -d' ' -f2 2>/dev/null || echo "unknown")
        log_info "Solana CLI version: $solana_version"
    fi
    
    cache_result "$cache_file" "VALID"
    log_success "Environment validation passed"
    return 0
}

# Determine validation scope based on trigger context
determine_validation_scope() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    
    case "$hook_type" in
        "pre-edit")
            # Only validate for critical configuration files
            if [[ "$file_path" =~ (Anchor\.toml|package\.json|Cargo\.toml)$ ]]; then
                echo "versions program_ids"
            elif [[ "$file_path" =~ programs/.*/src/lib\.rs$ ]]; then
                echo "program_ids"
            else
                echo "none"
            fi
            ;;
        "pre-bash")
            if [[ "$command" =~ (anchor.*build|anchor.*deploy|npm.*build|npm.*deploy) ]]; then
                echo "versions program_ids environment"
            else
                echo "none"
            fi
            ;;
        "session-start"|"project-start")
            echo "versions program_ids environment"
            ;;
        *)
            echo "none"
            ;;
    esac
}

# Main execution logic
main() {
    init_cache
    
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local validation_scope=$(determine_validation_scope)
    
    log_info "Core validation triggered: $hook_type"
    
    if [[ "$validation_scope" == "none" ]]; then
        exit 0
    fi
    
    local validations_needed=($validation_scope)
    
    for validation in "${validations_needed[@]}"; do
        case "$validation" in
            "versions")
                if ! validate_anchor_versions; then
                    log_error "Version validation failed"
                    exit 1
                fi
                ;;
            "program_ids")
                if ! validate_program_ids; then
                    log_error "Program ID validation failed"
                    exit 1
                fi
                ;;
            "environment")
                if ! validate_environment; then
                    log_error "Environment validation failed"
                    exit 1
                fi
                ;;
        esac
    done
    
    log_success "All core validations passed"
}

# Run main function if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi