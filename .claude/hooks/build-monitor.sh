#!/bin/bash

# Build Monitor Hook - Lightweight Build Process Monitoring
# Replaces: continuous-idl-monitor.sh, mcp-schema-validator.sh, ide-diagnostics-guard.sh, solana-validator.sh (partially)
# Triggers: pre-bash (build commands only), session-start (minimal)

set -e

# Cache configuration
CACHE_DIR=".claude/cache"
CACHE_EXPIRY=600  # 10 minutes (longer for build validations)

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [BUILD-MONITOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [BUILD-MONITOR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [BUILD-MONITOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [BUILD-MONITOR]${NC} $1"
}

# Cache management
init_cache() {
    mkdir -p "$CACHE_DIR"
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

# Quick compilation health check with caching
check_compilation_health() {
    local cache_file="$CACHE_DIR/compilation_health_cache"
    
    if is_cache_valid "$cache_file"; then
        local cached_result=$(cat "$cache_file" 2>/dev/null || echo "")
        if [[ "$cached_result" == "HEALTHY" ]]; then
            return 0
        fi
    fi
    
    log_info "Quick compilation health check..."
    
    # Lightweight check - just test if cargo can parse the workspace
    if command -v cargo >/dev/null 2>&1; then
        if ! cargo metadata --no-deps --quiet >/dev/null 2>&1; then
            cache_result "$cache_file" "UNHEALTHY"
            log_error "Compilation metadata check failed - there may be syntax errors"
            log_error "Run 'cargo check --workspace' for detailed errors"
            return 1
        fi
    fi
    
    cache_result "$cache_file" "HEALTHY"
    log_success "Compilation health check passed"
    return 0
}

# Pre-build dependency validation with caching
validate_build_dependencies() {
    local cache_file="$CACHE_DIR/build_deps_cache"
    
    if is_cache_valid "$cache_file"; then
        return 0
    fi
    
    log_info "Validating build dependencies..."
    
    # Check critical build tools
    local missing_tools=()
    
    if ! command -v anchor >/dev/null 2>&1; then
        missing_tools+=("anchor")
    fi
    
    if ! command -v cargo >/dev/null 2>&1; then
        missing_tools+=("cargo")
    fi
    
    if [[ -f "package.json" ]] && ! command -v node >/dev/null 2>&1; then
        missing_tools+=("node")
    fi
    
    if [[ ${#missing_tools[@]} -gt 0 ]]; then
        cache_result "$cache_file" "MISSING_TOOLS"
        log_error "Missing required build tools: ${missing_tools[*]}"
        return 1
    fi
    
    # Quick solana CLI check for deployment commands
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    if [[ "$command" =~ deploy ]] && ! command -v solana >/dev/null 2>&1; then
        log_warning "Solana CLI not found - deployment may fail"
        log_warning "Install Solana CLI for deployment operations"
    fi
    
    cache_result "$cache_file" "VALID"
    log_success "Build dependencies validated"
    return 0
}

# MCP integration validation with caching (lightweight)
validate_mcp_integration() {
    local cache_file="$CACHE_DIR/mcp_integration_cache"
    
    if is_cache_valid "$cache_file"; then
        return 0
    fi
    
    # Check if MCP tools are being used in scripts
    if [[ -d "scripts" ]] && grep -r "mcp__crypto-lottery-solana" scripts/ >/dev/null 2>&1; then
        log_info "MCP integration detected in scripts"
        
        # Verify basic MCP tool availability is documented
        if [[ -f "CLAUDE.md" ]] && grep -q "mcp__crypto-lottery-solana" "CLAUDE.md"; then
            log_success "MCP tools documented in project memory"
        else
            log_warning "MCP tools used but not documented in CLAUDE.md"
        fi
    fi
    
    cache_result "$cache_file" "CHECKED"
    return 0
}

# Stack overflow prevention for Solana programs
check_stack_usage_patterns() {
    local cache_file="$CACHE_DIR/stack_patterns_cache"
    
    # Only run this check if Rust program files have changed recently
    local recent_rust_changes=$(find programs -name "*.rs" -mmin -10 2>/dev/null | wc -l)
    if [[ $recent_rust_changes -eq 0 ]] && is_cache_valid "$cache_file"; then
        return 0
    fi
    
    log_info "Checking for potential stack overflow patterns..."
    
    # Look for large data structures being passed by value
    local large_structs_found=0
    
    # Find instruction structs with many fields
    for rust_file in $(find programs -name "*.rs" 2>/dev/null); do
        local field_count=$(grep -A 30 "#\[derive(Accounts)\]" "$rust_file" 2>/dev/null | grep -c "pub.*:" 2>/dev/null || echo 0)
        field_count=$(echo "$field_count" | head -1 | tr -d '\n' | tr -d ' ')
        field_count=${field_count:-0}
        if [[ "$field_count" =~ ^[0-9]+$ ]] && [[ $field_count -gt 30 ]]; then
            log_warning "Large instruction struct in $(basename "$rust_file"): $field_count fields"
            log_warning "Consider using Box<> or splitting into smaller structs"
            large_structs_found=$((large_structs_found + 1))
        fi
    done
    
    if [[ $large_structs_found -eq 0 ]]; then
        cache_result "$cache_file" "NO_ISSUES"
        log_success "No stack overflow patterns detected"
    else
        cache_result "$cache_file" "WARNINGS_FOUND"
        log_warning "Found $large_structs_found potential stack usage concerns"
    fi
    
    return 0
}

# Determine monitoring scope based on command
determine_monitoring_scope() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    
    case "$hook_type" in
        "pre-bash")
            if [[ "$command" =~ anchor[[:space:]]+build ]]; then
                echo "compilation deps stack"
            elif [[ "$command" =~ anchor[[:space:]]+deploy ]]; then
                echo "compilation deps"
            elif [[ "$command" =~ npm[[:space:]]+run[[:space:]].*(build|deploy) ]]; then
                echo "compilation deps"
            else
                echo "none"
            fi
            ;;
        "session-start"|"project-start")
            echo "deps mcp"
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
    local monitoring_scope=$(determine_monitoring_scope)
    
    if [[ "$monitoring_scope" == "none" ]]; then
        exit 0
    fi
    
    log_info "Build monitoring activated: $hook_type"
    
    local checks_needed=($monitoring_scope)
    
    for check in "${checks_needed[@]}"; do
        case "$check" in
            "compilation")
                if ! check_compilation_health; then
                    log_error "Compilation health check failed"
                    exit 1
                fi
                ;;
            "deps")
                if ! validate_build_dependencies; then
                    log_error "Build dependency validation failed"
                    exit 1
                fi
                ;;
            "stack")
                check_stack_usage_patterns  # Warning only, don't exit
                ;;
            "mcp")
                validate_mcp_integration  # Info only, don't exit
                ;;
        esac
    done
    
    # IDL Generation Post-Build Validation
    validate_post_build_idl_status
    
    log_success "Build monitoring completed"
}

# Validate IDL status after build operations
validate_post_build_idl_status() {
    # Only run for build-related commands
    local command="${CLAUDE_BASH_COMMAND:-}"
    if [[ "$command" != *"anchor build"* && "$command" != *"npm run build"* && "$command" != *"cargo build"* ]]; then
        return 0
    fi
    
    log_info "Validating post-build IDL status..."
    
    # Check if IDL directory was created
    if [[ ! -d "target/idl" ]]; then
        log_warning "⚠️  No target/idl directory after build"
        log_warning "   IDL generation may have failed"
        return 0
    fi
    
    # Check if IDL files were generated
    local idl_files=$(find target/idl -name "*.json" 2>/dev/null | wc -l | tr -d ' ')
    if [[ $idl_files -eq 0 ]]; then
        log_error "🔴 CRITICAL: No IDL files generated after build"
        log_error "   This indicates IDL generation failure"
        log_error "   Try: npm run build:reliable"
        return 1
    fi
    
    # Validate IDL file sizes (should not be tiny)
    local invalid_idls=0
    for idl_file in target/idl/*.json; do
        if [[ -f "$idl_file" ]]; then
            local size=$(stat -f%z "$idl_file" 2>/dev/null || echo "0")
            if [[ $size -lt 1000 ]]; then  # Less than 1KB is suspicious
                log_warning "⚠️  IDL file seems too small: $(basename "$idl_file") (${size}B)"
                invalid_idls=$((invalid_idls + 1))
            fi
        fi
    done
    
    if [[ $invalid_idls -gt 0 ]]; then
        log_warning "⚠️  $invalid_idls IDL files appear invalid (too small)"
        log_warning "   Consider rebuilding with: npm run build:reliable"
    fi
    
    log_success "IDL validation completed ($idl_files files found)"
    return 0
}

# Run main function if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi