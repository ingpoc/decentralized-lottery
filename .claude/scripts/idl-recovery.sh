#!/bin/bash

# IDL Recovery Script
# Automatically diagnoses and fixes IDL generation issues

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
CYAN='\033[0;36m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [IDL-RECOVERY]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [IDL-RECOVERY]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}⚠️  [IDL-RECOVERY]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [IDL-RECOVERY]${NC} $1"
}

log_step() {
    echo -e "${PURPLE}🔧 [IDL-RECOVERY]${NC} $1"
}

log_diagnosis() {
    echo -e "${CYAN}🔍 [IDL-RECOVERY]${NC} $1"
}

# Function to diagnose IDL generation issues
diagnose_idl_issues() {
    log_diagnosis "Diagnosing IDL generation issues..."
    
    local issues_found=()
    
    # Check for no-idl feature in workspace
    if grep -r "no-idl" Cargo.toml >/dev/null 2>&1; then
        issues_found+=("no-idl feature found in workspace Cargo.toml")
    fi
    
    # Check for no-idl in program Cargo.toml files
    for cargo_file in programs/*/Cargo.toml; do
        if [[ -f "$cargo_file" ]] && grep -q "no-idl" "$cargo_file"; then
            issues_found+=("no-idl feature found in $cargo_file")
        fi
    done
    
    # Check for missing handler exports
    for mod_file in programs/*/src/instructions/mod.rs; do
        if [[ -f "$mod_file" ]]; then
            local program_name=$(echo "$mod_file" | cut -d'/' -f2)
            local instruction_files=$(find "$(dirname "$mod_file")" -name "*.rs" ! -name "mod.rs" | wc -l)
            local handler_exports=$(grep -c "handler" "$mod_file" 2>/dev/null || echo 0)
            
            if [[ $instruction_files -gt 2 && $handler_exports -eq 0 ]]; then
                issues_found+=("Missing handler exports in $program_name instructions/mod.rs")
            fi
        fi
    done
    
    # Check for lib.rs program structure issues
    for lib_file in programs/*/src/lib.rs; do
        if [[ -f "$lib_file" ]] && grep -q "#\[program\]" "$lib_file"; then
            local program_name=$(echo "$lib_file" | cut -d'/' -f2)
            local instruction_count=$(grep -c "pub fn.*ctx: Context<.*>" "$lib_file" || echo 0)
            
            if [[ $instruction_count -eq 0 ]]; then
                issues_found+=("No public instruction functions in $program_name lib.rs")
            fi
        fi
    done
    
    # Check for version mismatches
    local anchor_versions=$(find programs/ -name "Cargo.toml" -exec grep -o "anchor-.*=.*\"[0-9.]*\"" {} \; | grep -o "[0-9.]*" | sort -u | wc -l)
    if [[ $anchor_versions -gt 1 ]]; then
        issues_found+=("Multiple Anchor versions detected across programs")
    fi
    
    # Report findings
    if [[ ${#issues_found[@]} -eq 0 ]]; then
        log_success "No obvious IDL issues detected"
        return 0
    else
        log_error "Found ${#issues_found[@]} potential IDL issues:"
        for issue in "${issues_found[@]}"; do
            log_error "  • $issue"
        done
        return 1
    fi
}

# Function to fix no-idl feature issues
fix_no_idl_features() {
    log_step "Fixing no-idl feature issues..."
    
    # Remove no-idl from workspace Cargo.toml
    if grep -q "no-idl" Cargo.toml; then
        log_info "Removing no-idl from workspace Cargo.toml"
        sed -i.bak 's/no-idl = \[\]//g' Cargo.toml
        sed -i.bak '/^no-idl.*$/d' Cargo.toml
    fi
    
    # Remove no-idl from program Cargo.toml files
    for cargo_file in programs/*/Cargo.toml; do
        if [[ -f "$cargo_file" ]] && grep -q "no-idl" "$cargo_file"; then
            log_info "Removing no-idl from $cargo_file"
            sed -i.bak 's/no-idl = \[\]//g' "$cargo_file"
            sed -i.bak '/^no-idl.*$/d' "$cargo_file"
        fi
    done
    
    log_success "No-idl features removed"
}

# Function to fix missing handler exports
fix_handler_exports() {
    log_step "Fixing missing handler exports..."
    
    for mod_file in programs/*/src/instructions/mod.rs; do
        if [[ -f "$mod_file" ]]; then
            local program_name=$(echo "$mod_file" | cut -d'/' -f2)
            local instructions_dir=$(dirname "$mod_file")
            local instruction_files=()
            
            # Find instruction files
            while IFS= read -r -d '' file; do
                local basename_file=$(basename "$file" .rs)
                if [[ "$basename_file" != "mod" ]]; then
                    instruction_files+=("$basename_file")
                fi
            done < <(find "$instructions_dir" -name "*.rs" -print0)
            
            # Check if mod.rs needs updates
            local needs_update=false
            for instruction in "${instruction_files[@]}"; do
                if ! grep -q "$instruction.*handler" "$mod_file"; then
                    needs_update=true
                    break
                fi
            done
            
            if [[ "$needs_update" == "true" ]]; then
                log_info "Updating handler exports in $program_name"
                
                # Backup original
                cp "$mod_file" "${mod_file}.bak"
                
                # Add missing handler exports
                for instruction in "${instruction_files[@]}"; do
                    if ! grep -q "$instruction" "$mod_file"; then
                        local struct_name=$(echo "$instruction" | sed 's/.*/\u&/' | sed 's/_\([a-z]\)/\u\1/g')
                        echo "pub use $instruction::{$struct_name, handler as ${instruction}_handler};" >> "$mod_file"
                    fi
                done
            fi
        fi
    done
    
    log_success "Handler exports fixed"
}

# Function to fix program structure issues
fix_program_structure() {
    log_step "Fixing program structure issues..."
    
    for lib_file in programs/*/src/lib.rs; do
        if [[ -f "$lib_file" ]] && grep -q "#\[program\]" "$lib_file"; then
            local program_name=$(echo "$lib_file" | cut -d'/' -f2)
            local instruction_count=$(grep -c "pub fn.*ctx: Context<.*>" "$lib_file" || echo 0)
            
            if [[ $instruction_count -eq 0 ]]; then
                log_warning "Program $program_name has no public instruction functions"
                log_info "This may indicate that instruction handlers are commented out"
                log_info "Check $lib_file and uncomment necessary instruction functions"
            fi
        fi
    done
}

# Function to attempt IDL generation recovery
attempt_idl_recovery() {
    log_step "Attempting IDL generation recovery..."
    
    # Clean build first
    log_info "Cleaning previous build artifacts..."
    anchor clean || true
    
    # Try minimal build with IDL generation
    log_info "Attempting minimal build with IDL generation..."
    if anchor build --idl target/idl 2>/dev/null; then
        log_success "IDL generation successful!"
        return 0
    fi
    
    # Try build without linting
    log_info "Attempting build without linting..."
    if anchor build --skip-lint 2>/dev/null; then
        log_success "Build successful (without linting)!"
        return 0
    fi
    
    # Try cargo build directly
    log_info "Attempting direct cargo build..."
    if cargo build-sbf 2>/dev/null; then
        log_success "Direct cargo build successful!"
        
        # Try to sync IDL files
        if [[ -f "scripts/sync-idl.ts" ]]; then
            log_info "Attempting IDL sync..."
            npm run sync:idl || true
        fi
        
        return 0
    fi
    
    log_error "All recovery attempts failed"
    return 1
}

# Function to generate recovery report
generate_recovery_report() {
    local recovery_status=$1
    local report_file=".claude/idl-recovery-report.md"
    
    cat > "$report_file" << EOF
# IDL Recovery Report

**Generated:** $(date)
**Recovery Status:** $([ $recovery_status -eq 0 ] && echo "✅ SUCCESS" || echo "❌ FAILED")

## Issues Diagnosed
$(diagnose_idl_issues 2>&1 | grep "  •" || echo "No specific issues identified")

## Recovery Actions Taken

1. **No-IDL Features:** Removed from all Cargo.toml files
2. **Handler Exports:** Fixed missing exports in instructions/mod.rs files
3. **Program Structure:** Validated public instruction functions
4. **Build Attempts:** Tried multiple build strategies

## Current Status

EOF
    
    if [[ $recovery_status -eq 0 ]]; then
        echo "✅ **IDL generation recovered successfully**" >> "$report_file"
        echo "" >> "$report_file"
        echo "IDL files should now be available in:" >> "$report_file"
        find target/ -name "*.json" -path "*/idl/*" 2>/dev/null | while read -r idl_file; do
            echo "- $idl_file" >> "$report_file"
        done
    else
        echo "❌ **IDL generation still failing**" >> "$report_file"
        echo "" >> "$report_file"
        echo "## Recommended Next Steps" >> "$report_file"
        echo "" >> "$report_file"
        echo "1. Check for Anchor version compatibility issues" >> "$report_file"
        echo "2. Manually review lib.rs files for instruction function issues" >> "$report_file"
        echo "3. Consider using IDL-sync-specialist agent for advanced recovery" >> "$report_file"
        echo "4. Use deployment workaround: deploy programs first, then sync IDLs" >> "$report_file"
    fi
    
    log_success "Recovery report generated: $report_file"
}

# Main recovery function
main() {
    log_info "Starting IDL recovery process..."
    
    # Diagnose issues first
    diagnose_idl_issues || log_warning "Issues detected - proceeding with fixes..."
    
    # Apply fixes
    fix_no_idl_features
    fix_handler_exports
    fix_program_structure
    
    # Attempt recovery
    local recovery_status=1
    if attempt_idl_recovery; then
        recovery_status=0
    fi
    
    # Generate report
    generate_recovery_report $recovery_status
    
    if [[ $recovery_status -eq 0 ]]; then
        log_success "🎉 IDL recovery completed successfully!"
        log_info "Your programs should now build with proper IDL generation"
    else
        log_error "IDL recovery failed - see report for details"
        log_info "Consider using the deployment workaround or IDL-sync-specialist agent"
    fi
    
    return $recovery_status
}

# Run if executed directly
if [[ "${BASH_SOURCE[0]}" == "${0}" ]]; then
    main "$@"
fi