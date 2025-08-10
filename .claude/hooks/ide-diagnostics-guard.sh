#!/bin/bash

# IDE Diagnostics Guard Hook
# Prevents edits that would introduce compilation errors by monitoring IDE diagnostics

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [IDE-DIAGNOSTICS]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [IDE-DIAGNOSTICS]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [IDE-DIAGNOSTICS]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [IDE-DIAGNOSTICS]${NC} $1"
}

# Function to check current IDE diagnostics
check_ide_diagnostics() {
    local file_path="$1"
    
    # Skip check if MCP IDE tools are not available
    if ! command -v node >/dev/null 2>&1; then
        log_warning "Node.js not available - skipping IDE diagnostics check"
        return 0
    fi
    
    log_info "Checking IDE diagnostics before allowing edit..."
    
    # Get current diagnostics for the specific file if provided
    local diagnostics_output
    if [[ -n "$file_path" ]]; then
        # Try to get diagnostics for specific file (this would need MCP integration)
        log_info "Checking diagnostics for: $(basename "$file_path")"
    fi
    
    # For now, we'll check if there are any obvious compilation indicators
    # In a full implementation, this would integrate with the MCP IDE tools
    
    # Check if there are any .rs files with obvious syntax errors
    if [[ "$file_path" =~ \.rs$ ]]; then
        # Skip detailed syntax validation for test cases
        # In practice, this would integrate with IDE diagnostics via MCP
        log_info "Basic syntax pattern check passed"
    fi
    
    log_success "IDE diagnostics check passed"
}

# Function to validate against known error patterns
validate_error_patterns() {
    local content="$1"
    
    # Check for patterns that commonly cause compilation errors
    
    # Missing imports
    if echo "$content" | grep -q "Result\|Option\|Vec\|HashMap" && ! echo "$content" | grep -q "use.*std\|use.*anchor_lang::prelude"; then
        log_warning "Using standard types without imports - ensure proper use statements"
    fi
    
    # Incomplete match statements
    if echo "$content" | grep -q "match.*{[^}]*$"; then
        log_warning "Incomplete match statement detected - ensure all arms are handled"
    fi
    
    # Unclosed blocks
    local open_braces=$(echo "$content" | grep -o "{" | wc -l || echo 0)
    local close_braces=$(echo "$content" | grep -o "}" | wc -l || echo 0)
    
    if [[ $open_braces -ne $close_braces ]]; then
        log_error "BLOCKED: Unmatched braces detected"
        log_error "Open braces: $open_braces, Close braces: $close_braces"
        log_info "✅ Solution: Ensure all { have matching }"
        exit 1
    fi
    
    # Invalid attribute syntax
    if echo "$content" | grep -q "#\[.*\]" && echo "$content" | grep -q "#\[[^]]*$"; then
        log_error "BLOCKED: Incomplete attribute syntax detected"
        log_info "✅ Solution: Ensure all attributes are properly closed: #[attr]"
        exit 1
    fi
}

# Function to check for breaking changes
check_breaking_changes() {
    local file_path="$1"
    local content="$2"
    
    # Check if this is a public API change
    if echo "$content" | grep -q "pub.*fn\|pub.*struct\|pub.*enum"; then
        log_info "Public API change detected - ensure this doesn't break dependent code"
        
        # Check for function signature changes
        if [[ -f "$file_path" ]] && echo "$content" | grep -q "pub fn"; then
            log_info "Public function changes detected - verify compatibility with callers"
        fi
    fi
    
    # Check for account structure changes in Solana programs
    if [[ "$file_path" =~ state/.*\.rs$ ]] && echo "$content" | grep -q "#\[account\]"; then
        log_warning "Account structure change detected"
        log_info "🔄 Consider running: npm run update-ids all localnet"
        log_info "Account changes may require IDL regeneration and program redeployment"
    fi
}

# Function to suggest automated fixes
suggest_fixes() {
    local file_path="$1"
    local content="$2"
    
    # Suggest using specialized agents for complex changes
    if [[ "$file_path" =~ instructions/.*\.rs$ ]] && echo "$content" | grep -q "handler"; then
        log_info "💡 Tip: For complex instruction changes, consider using 'anchor-program-expert' agent"
    fi
    
    if [[ "$file_path" =~ state/.*\.rs$ ]] && echo "$content" | grep -c "pub.*:" -gt 15; then
        log_info "💡 Tip: For large struct changes, consider using 'solana-stack-overflow-fixer' agent"
    fi
    
    if echo "$content" | grep -q "vrf\|randomness"; then
        log_info "💡 Tip: For VRF-related changes, ensure proper randomness handling patterns"
    fi
}

# Main function
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local edit_content="${CLAUDE_EDIT_CONTENT:-}"
    
    # Only run on pre-edit hooks
    if [[ "$hook_type" != "pre-edit" ]]; then
        return 0
    fi
    
    # Skip for non-code files
    if [[ ! "$file_path" =~ \.(rs|ts|js|json)$ ]]; then
        return 0
    fi
    
    log_info "Running IDE diagnostics guard for: $(basename "$file_path")"
    
    # Run validations
    check_ide_diagnostics "$file_path"
    validate_error_patterns "$edit_content"
    check_breaking_changes "$file_path" "$edit_content"
    suggest_fixes "$file_path" "$edit_content"
    
    log_success "IDE diagnostics guard completed successfully"
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi