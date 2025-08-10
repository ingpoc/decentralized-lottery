#!/bin/bash

# Continuous IDL Monitor Hook
# Runs automated IDL health checks and prevents common failures

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [IDL-MONITOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [IDL-MONITOR]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [IDL-MONITOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [IDL-MONITOR]${NC} $1"
}

log_check() {
    echo -e "${PURPLE}🔍 [IDL-MONITOR]${NC} $1"
}

# Function to test IDL generation before build
test_idl_generation() {
    log_check "Testing IDL generation capability..."
    
    # Test if anchor build would succeed (dry run)
    if command -v anchor >/dev/null 2>&1; then
        # Check if programs can be parsed by anchor
        if ! anchor parse &>/dev/null; then
            log_error "Programs cannot be parsed by Anchor - IDL generation will fail"
            log_info "Common causes: missing handler exports, syntax errors, import issues"
            return 1
        fi
        
        # Check if IDL files exist and are valid JSON
        for program_dir in programs/*/; do
            program_name=$(basename "$program_dir")
            idl_path="target/idl/${program_name//-/_}.json"
            
            if [[ -f "$idl_path" ]]; then
                if ! jq empty "$idl_path" &>/dev/null; then
                    log_error "Invalid IDL JSON detected: $idl_path"
                    return 1
                fi
                log_success "IDL validation passed: $program_name"
            else
                log_warning "IDL not found (will be generated): $program_name"
            fi
        done
    else
        log_warning "Anchor CLI not found - skipping IDL generation test"
    fi
    
    return 0
}

# Function to validate program structure health
validate_program_health() {
    log_check "Validating program structure health..."
    
    # Check for required files in each program
    for program_dir in programs/*/; do
        program_name=$(basename "$program_dir")
        log_info "Checking $program_name structure..."
        
        # Check for lib.rs
        if [[ ! -f "$program_dir/src/lib.rs" ]]; then
            log_error "Missing lib.rs in $program_name"
            return 1
        fi
        
        # Check for instruction modules
        if [[ -d "$program_dir/src/instructions" ]]; then
            if [[ ! -f "$program_dir/src/instructions/mod.rs" ]]; then
                log_error "Missing instructions/mod.rs in $program_name"
                return 1
            fi
            
            # Count instruction files vs exports
            local rs_files=$(find "$program_dir/src/instructions" -name "*.rs" ! -name "mod.rs" | wc -l)
            local mod_exports=$(grep -c "pub use" "$program_dir/src/instructions/mod.rs" 2>/dev/null || echo 0)
            
            if [[ $rs_files -gt 0 && $mod_exports -eq 0 ]]; then
                log_error "Instruction files exist but no exports in mod.rs for $program_name"
                return 1
            fi
            
            log_success "Instruction structure valid: $program_name"
        fi
        
        # Check for state modules
        if [[ -d "$program_dir/src/state" ]]; then
            if [[ ! -f "$program_dir/src/state/mod.rs" ]]; then
                log_warning "Missing state/mod.rs in $program_name - may cause import issues"
            fi
        fi
    done
    
    return 0
}

# Function to check for common IDL-breaking patterns in codebase
scan_idl_breaking_patterns() {
    log_check "Scanning for IDL-breaking patterns..."
    
    local issues_found=0
    
    # Check for missing handler exports
    for mod_file in programs/*/src/instructions/mod.rs; do
        if [[ -f "$mod_file" ]]; then
            local program_name=$(echo "$mod_file" | cut -d'/' -f2)
            local instruction_files=$(find "$(dirname "$mod_file")" -name "*.rs" ! -name "mod.rs" | wc -l)
            local handler_exports=$(grep -c "handler" "$mod_file" 2>/dev/null || echo 0)
            
            if [[ $instruction_files -gt 2 && $handler_exports -eq 0 ]]; then
                log_error "Missing handler exports in $program_name instructions/mod.rs"
                ((issues_found++))
            fi
        fi
    done
    
    # Check for conflicting struct names across codebase
    local struct_conflicts=$(find programs/ -name "*.rs" -exec grep -l "pub struct.*Account" {} \; | \
                           xargs grep "pub struct" | \
                           cut -d':' -f2 | \
                           sort | uniq -d | wc -l)
    
    if [[ $struct_conflicts -gt 0 ]]; then
        log_warning "Potential struct name conflicts detected"
        ((issues_found++))
    fi
    
    # Check for version mismatches in Cargo.toml files
    for cargo_file in programs/*/Cargo.toml; do
        if [[ -f "$cargo_file" ]]; then
            local anchor_versions=$(grep -o "anchor-.*=.*\"[0-9.]*\"" "$cargo_file" | grep -o "[0-9.]*" | sort -u | wc -l)
            if [[ $anchor_versions -gt 1 ]]; then
                log_error "Version mismatch detected in $(basename "$(dirname "$cargo_file")")"
                ((issues_found++))
            fi
        fi
    done
    
    if [[ $issues_found -eq 0 ]]; then
        log_success "No IDL-breaking patterns detected"
    else
        log_warning "Found $issues_found potential IDL issues"
    fi
    
    return $issues_found
}

# Function to generate IDL health report
generate_health_report() {
    log_info "Generating IDL health report..."
    
    local report_file=".claude/idl-health-report.md"
    
    cat > "$report_file" << EOF
# IDL Health Report

**Generated:** $(date)
**Status:** $(test_idl_generation && echo "✅ HEALTHY" || echo "❌ ISSUES DETECTED")

## Program Status

EOF
    
    for program_dir in programs/*/; do
        local program_name=$(basename "$program_dir")
        echo "### $program_name" >> "$report_file"
        
        # Check instruction count
        local instruction_count=$(find "$program_dir/src/instructions" -name "*.rs" ! -name "mod.rs" 2>/dev/null | wc -l || echo 0)
        echo "- **Instructions:** $instruction_count" >> "$report_file"
        
        # Check export count
        local export_count=$(grep -c "pub use" "$program_dir/src/instructions/mod.rs" 2>/dev/null || echo 0)
        echo "- **Exports:** $export_count" >> "$report_file"
        
        # IDL status
        local idl_path="target/idl/${program_name//-/_}.json"
        if [[ -f "$idl_path" ]]; then
            local idl_size=$(stat -f%z "$idl_path" 2>/dev/null || echo 0)
            echo "- **IDL Size:** ${idl_size} bytes" >> "$report_file"
            echo "- **IDL Status:** ✅ Generated" >> "$report_file"
        else
            echo "- **IDL Status:** ❌ Missing" >> "$report_file"
        fi
        
        echo "" >> "$report_file"
    done
    
    echo "## Recommendations" >> "$report_file"
    echo "" >> "$report_file"
    echo "- Run \`anchor build\` to regenerate IDLs" >> "$report_file"
    echo "- Use \`npm run update-ids all localnet\` to sync frontend" >> "$report_file"
    echo "- Check hooks are preventing IDL issues: \`.claude/hooks/idl-validation-guard.sh\`" >> "$report_file"
    
    log_success "IDL health report generated: $report_file"
}

# Function to suggest automated fixes
suggest_automated_fixes() {
    log_info "Suggesting automated fixes for detected issues..."
    
    # Check if there are instruction files without handler exports
    for program_dir in programs/*/; do
        local program_name=$(basename "$program_dir")
        local instructions_dir="$program_dir/src/instructions"
        
        if [[ -d "$instructions_dir" ]]; then
            local mod_file="$instructions_dir/mod.rs"
            local instruction_files=()
            
            # Find instruction files
            while IFS= read -r -d '' file; do
                local basename_file=$(basename "$file" .rs)
                if [[ "$basename_file" != "mod" ]]; then
                    instruction_files+=("$basename_file")
                fi
            done < <(find "$instructions_dir" -name "*.rs" -print0)
            
            # Check if mod.rs has exports for each instruction
            if [[ -f "$mod_file" ]] && [[ ${#instruction_files[@]} -gt 0 ]]; then
                for instruction in "${instruction_files[@]}"; do
                    if ! grep -q "$instruction" "$mod_file"; then
                        log_warning "Missing export for $instruction in $program_name"
                        log_info "💡 Add: pub use $instruction::{$(echo "$instruction" | sed 's/.*/\u&/')Context, handler as ${instruction}_handler};"
                    fi
                done
            fi
        fi
    done
}

# Main monitoring function
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    
    log_info "Running continuous IDL monitoring..."
    
    # Run health checks
    if ! validate_program_health; then
        log_error "Program health check failed - IDL generation likely to fail"
        exit 1
    fi
    
    # Test IDL generation capability
    if ! test_idl_generation; then
        log_error "IDL generation test failed - build will likely fail"
        exit 1
    fi
    
    # Scan for breaking patterns
    scan_idl_breaking_patterns
    
    # Generate health report
    generate_health_report
    
    # Suggest fixes if in pre-build context
    if [[ "$hook_type" == "pre-bash" ]] && [[ "${CLAUDE_BASH_COMMAND:-}" =~ anchor.*build|npm.*build ]]; then
        suggest_automated_fixes
    fi
    
    log_success "IDL monitoring completed - no blocking issues detected"
}

# Run if executed directly  
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi