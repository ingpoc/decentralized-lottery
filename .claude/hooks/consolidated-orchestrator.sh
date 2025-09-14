#!/bin/bash

# Consolidated Hook Orchestrator
# Intelligently routes hook calls to the appropriate consolidated validators
# Replaces: master-validator.sh with optimized routing

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
NC='\033[0m'

log_info() {
    echo -e "${BLUE}ℹ️  [ORCHESTRATOR]${NC} $1"
}

log_success() {
    echo -e "${GREEN}✅ [ORCHESTRATOR]${NC} $1"
}

log_error() {
    echo -e "${RED}🚨 [ORCHESTRATOR]${NC} $1"
}

# Get the directory where this script is located
HOOKS_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" &> /dev/null && pwd )"

# Intelligent routing logic
route_validation() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    local file_path="${CLAUDE_FILE_PATH:-}"
    local command="${CLAUDE_COMMAND:-${CLAUDE_BASH_COMMAND:-}}"
    
    local validators_needed=()
    
    case "$hook_type" in
        "pre-edit")
            # Route based on file type
            if [[ "$file_path" =~ (Anchor\.toml|package\.json|Cargo\.toml)$ ]]; then
                validators_needed+=("core-validator")
            fi
            
            if [[ "$file_path" =~ \.rs$ ]] && [[ "$file_path" =~ programs/ ]]; then
                validators_needed+=("core-validator" "idl-guard")
            fi
            ;;
            
        "pre-bash")
            # Route based on command type
            if [[ "$command" =~ (anchor.*build|anchor.*deploy|npm.*(build|deploy)) ]]; then
                validators_needed+=("core-validator" "idl-guard" "build-monitor")
            elif [[ "$command" =~ anchor.*idl ]]; then
                validators_needed+=("idl-guard")
            fi
            ;;
            
        "session-start"|"project-start")
            validators_needed+=("core-validator" "idl-guard" "build-monitor")
            ;;
    esac
    
    echo "${validators_needed[@]}"
}

# Execute validators in parallel for efficiency
execute_validators() {
    local validators=("$@")
    local pids=()
    local results=()
    
    if [[ ${#validators[@]} -eq 0 ]]; then
        log_info "No validation needed for this operation"
        return 0
    fi
    
    log_info "Running ${#validators[@]} validator(s): ${validators[*]}"
    
    # Start validators in parallel
    for validator in "${validators[@]}"; do
        (
            export CLAUDE_HOOK_TYPE CLAUDE_FILE_PATH CLAUDE_COMMAND CLAUDE_BASH_COMMAND CLAUDE_EDIT_CONTENT
            "$HOOKS_DIR/$validator.sh"
        ) &
        pids+=($!)
    done
    
    # Wait for all validators to complete
    local exit_code=0
    for i in "${!pids[@]}"; do
        wait "${pids[$i]}"
        local result=$?
        results+=($result)
        if [[ $result -ne 0 ]]; then
            exit_code=1
            log_error "${validators[$i]} failed with code $result"
        fi
    done
    
    return $exit_code
}

# Main orchestration logic
main() {
    local hook_type="${CLAUDE_HOOK_TYPE:-unknown}"
    
    log_info "Consolidated validation orchestrator started: $hook_type"
    
    local validators_needed=($(route_validation))
    
    if ! execute_validators "${validators_needed[@]}"; then
        log_error "Validation failed"
        exit 1
    fi
    
    if [[ ${#validators_needed[@]} -gt 0 ]]; then
        log_success "All validations passed efficiently"
    fi
}

# Run main function
main "$@"