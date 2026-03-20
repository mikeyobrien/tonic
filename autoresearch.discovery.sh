#!/bin/bash
# autoresearch.discovery.sh - Phase 1: Automated Gap Analysis
# 
# This script performs automated discovery of:
# 1. Missing host function registrations
# 2. Native backend parity gaps
# 3. Error message quality issues
# 4. Example coverage gaps
# 5. Documentation vs. implementation mismatches

set -e

PROJECT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$PROJECT_DIR"

echo "========================================"
echo "Tonic Autoresearch - Phase 1 Discovery"
echo "========================================"
echo ""

# Output files
GAP_REPORT="autoresearch.gap-report.md"
MISSING_HOST_FUNCS="autoresearch.missing-host-funcs.txt"
NATIVE_PARITY="autoresearch.native-parity.csv"
ERROR_MESSAGES="autoresearch.error-messages.md"
EXAMPLE_COVERAGE="autoresearch.example-coverage.md"
DOC_MISMATCH="autoresearch.doc-mismatch.md"

# Initialize output files
echo "# Tonic Autoresearch - Gap Analysis Report" > "$GAP_REPORT"
echo "" >> "$GAP_REPORT"
echo "Generated: $(date)" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "module,function,interpreter,native,notes" > "$NATIVE_PARITY"

echo "# Error Message Quality Audit" > "$ERROR_MESSAGES"
echo "" >> "$ERROR_MESSAGES"

echo "# Example Coverage Report" > "$EXAMPLE_COVERAGE"
echo "" >> "$EXAMPLE_COVERAGE"

echo "# Documentation vs. Implementation Mismatch" > "$DOC_MISMATCH"
echo "" >> "$DOC_MISMATCH"

# ============================================================================
# SECTION 1: Host Function Registration Audit
# ============================================================================
echo "=== SECTION 1: Host Function Registration Audit ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "Analyzing host function registrations..."

# Extract all host_call atoms from manifest_stdlib.rs
echo "Extracting host calls from manifest_stdlib.rs..."
grep -oP 'host_call\(:[\w_]+\)' src/manifest_stdlib.rs 2>/dev/null | \
    sed 's/host_call(://g' | sed 's/)//g' | \
    sort -u > /tmp/all-host-calls.txt

echo "Found $(wc -l < /tmp/all-host-calls.txt) unique host calls in manifest_stdlib.rs"

# Extract all registered functions from interop modules
echo "Extracting registered functions from interop modules..."
> /tmp/registered-funcs.txt

for file in src/interop/*.rs; do
    if [ -f "$file" ]; then
        # Look for registry.register calls with function names
        grep -oP 'registry\.register\([^)]+name:\s*"[^"]+"' "$file" 2>/dev/null | \
            sed 's/.*name:\s*"\([^"]*\)".*/\1/' >> /tmp/registered-funcs.txt
        
        # Also look for simpler patterns
        grep -oP 'register\("[^"]+"' "$file" 2>/dev/null | \
            sed 's/register("//g' | sed 's/)//g' >> /tmp/registered-funcs.txt
    fi
done

# Also check interop.rs main file
if [ -f "src/interop.rs" ]; then
    grep -oP 'register\("[^"]+"' src/interop.rs 2>/dev/null | \
        sed 's/register("//g' | sed 's/)//g' >> /tmp/registered-funcs.txt
fi

sort -u /tmp/registered-funcs.txt > /tmp/registered-funcs-sorted.txt
mv /tmp/registered-funcs-sorted.txt /tmp/registered-funcs.txt

echo "Found $(wc -l < /tmp/registered-funcs.txt) unique registered functions"

# Find gaps
echo "Comparing host calls vs. registered functions..."
comm -23 /tmp/all-host-calls.txt /tmp/registered-funcs.txt > "$MISSING_HOST_FUNCS"

echo "" >> "$GAP_REPORT"
echo "### Host Function Registration Status" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"
echo "- **Total host calls in manifest**: $(wc -l < /tmp/all-host-calls.txt)" >> "$GAP_REPORT"
echo "- **Total registered functions**: $(wc -l < /tmp/registered-funcs.txt)" >> "$GAP_REPORT"
echo "- **Missing registrations**: $(wc -l < "$MISSING_HOST_FUNCS")" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

if [ -s "$MISSING_HOST_FUNCS" ]; then
    echo "### Missing Host Function Registrations" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    echo "```" >> "$GAP_REPORT"
    cat "$MISSING_HOST_FUNCS" >> "$GAP_REPORT"
    echo "```" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    echo "⚠️  **ACTION REQUIRED**: These host functions are called in manifest_stdlib.rs but not registered in interop modules" >> "$GAP_REPORT"
else
    echo "✅ All host calls appear to be registered" >> "$GAP_REPORT"
fi

echo "→ Output: $MISSING_HOST_FUNCS"

# ============================================================================
# SECTION 2: Native Backend Parity Check
# ============================================================================
echo "" >> "$GAP_REPORT"
echo "=== SECTION 2: Native Backend Parity Check ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "Checking native backend parity..."

# Check if c_backend/stubs.rs exists and has host call handlers
if [ -f "src/c_backend/stubs.rs" ]; then
    echo "Found c_backend/stubs.rs"
    
    # Count host call handlers
    host_handler_count=$(grep -c "fn host_" src/c_backend/stubs.rs 2>/dev/null || echo "0")
    echo "Found $host_handler_count host function handlers in stubs.rs"
    
    echo "" >> "$GAP_REPORT"
    echo "### Native Backend Handlers" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    echo "- **Host handlers in stubs.rs**: $host_handler_count" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    
    # List the handlers
    echo "Registered handlers:" >> "$GAP_REPORT"
    echo "```" >> "$GAP_REPORT"
    grep -oP 'fn host_[\w_]+' src/c_backend/stubs.rs 2>/dev/null | sort -u >> "$GAP_REPORT" || echo "(none found)" >> "$GAP_REPORT"
    echo "```" >> "$GAP_REPORT"
else
    echo "⚠️  src/c_backend/stubs.rs not found"
    echo "" >> "$GAP_REPORT"
    echo "⚠️  **CRITICAL**: src/c_backend/stubs.rs not found - native backend may not be implemented" >> "$GAP_REPORT"
fi

# Test native compilation (if tonic binary exists)
echo "" >> "$GAP_REPORT"
echo "### Native Compilation Test" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

if command -v tonic &> /dev/null; then
    echo "Testing native compilation with simple example..."
    
    # Create a simple test file
    cat > /tmp/native_test.tn << 'EOF'
defmodule NativeTest do
  def run do
    IO.puts("Hello from native!")
    List.first([1, 2, 3])
  end
end

NativeTest.run()
EOF
    
    if tonic compile /tmp/native_test.tn -o /tmp/native_test 2>/dev/null; then
        echo "✅ Native compilation successful" >> "$GAP_REPORT"
        echo "- **Compilation**: PASS" >> "$GAP_REPORT"
        
        if [ -x /tmp/native_test ]; then
            output=$(/tmp/native_test 2>&1 || true)
            echo "- **Execution**: $(if echo "$output" | grep -q "Hello from native"; then echo "PASS"; else echo "FAIL ($output)"; fi)" >> "$GAP_REPORT"
        fi
    else
        echo "❌ Native compilation failed" >> "$GAP_REPORT"
        echo "- **Compilation**: FAIL" >> "$GAP_REPORT"
    fi
else
    echo "⚠️  tonic binary not found in PATH - skipping native compilation test"
    echo "⚠️  **SKIPPED**: tonic binary not available for native parity testing" >> "$GAP_REPORT"
fi

echo "→ Output: $NATIVE_PARITY"

# ============================================================================
# SECTION 3: Error Message Collection
# ============================================================================
echo "" >> "$GAP_REPORT"
echo "=== SECTION 3: Error Message Quality Audit ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "Collecting error messages..."

# Run tests and capture errors
if command -v cargo &> /dev/null; then
    echo "Running cargo test to capture error messages..."
    
    # Capture test output
    cargo test 2>&1 | tee /tmp/test_output.log | head -100
    
    # Extract error messages
    grep -A3 "error:" /tmp/test_output.log 2>/dev/null | head -50 >> "$ERROR_MESSAGES" || echo "(no errors found in test output)" >> "$ERROR_MESSAGES"
    
    # Count errors
    error_count=$(grep -c "error:" /tmp/test_output.log 2>/dev/null || echo "0")
    
    echo "" >> "$GAP_REPORT"
    echo "### Test Error Summary" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    echo "- **Total errors in test suite**: $error_count" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    
    if [ "$error_count" -gt 0 ]; then
        echo "⚠️  Test suite has $error_count errors - review $ERROR_MESSAGES for details" >> "$GAP_REPORT"
    else
        echo "✅ Test suite passes with no errors" >> "$GAP_REPORT"
    fi
else
    echo "⚠️  cargo not found - skipping test suite error collection"
    echo "⚠️  **SKIPPED**: cargo not available for error message collection" >> "$GAP_REPORT"
fi

echo "→ Output: $ERROR_MESSAGES"

# ============================================================================
# SECTION 4: Example Coverage Analysis
# ============================================================================
echo "" >> "$GAP_REPORT"
echo "=== SECTION 4: Example Coverage Analysis ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "Analyzing example app coverage..."

# Count example apps
if [ -d "examples/apps" ]; then
    app_count=$(find examples/apps -maxdepth 1 -type d | wc -l)
    app_count=$((app_count - 1))  # Subtract 1 for the "." directory
    
    echo "Found $app_count example apps in examples/apps/"
    
    # Check which stdlib modules are used
    echo "" >> "$GAP_REPORT"
    echo "### Stdlib Module Usage in Examples" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    
    for module in System String Path IO List Map Enum; do
        usage_count=$(find examples/apps -name "*.tn" -exec grep -l "$module\\." {} \; 2>/dev/null | wc -l)
        echo "- **$module**: $usage_count example apps" >> "$GAP_REPORT"
        
        # List the apps using this module
        if [ "$usage_count" -gt 0 ]; then
            echo "  - Used in:" >> "$GAP_REPORT"
            find examples/apps -name "*.tn" -exec grep -l "$module\\." {} \; 2>/dev/null | \
                sed 's|examples/apps/||' | sed 's|/.*||' | sort -u | \
                while read app; do echo "    - $app" >> "$EXAMPLE_COVERAGE"; done
        fi
    done
    
    echo "" >> "$GAP_REPORT"
    
    # Identify uncovered modules
    uncovered_modules=$(for module in System String Path IO List Map Enum; do
        usage_count=$(find examples/apps -name "*.tn" -exec grep -l "$module\\." {} \; 2>/dev/null | wc -l)
        if [ "$usage_count" -eq 0 ]; then
            echo "$module"
        fi
    done)
    
    if [ -n "$uncovered_modules" ]; then
        echo "⚠️  **Modules with NO example coverage**:" >> "$GAP_REPORT"
        echo "" >> "$GAP_REPORT"
        echo "$uncovered_modules" | while read module; do
            echo "- $module" >> "$GAP_REPORT"
        done
    else
        echo "✅ All stdlib modules have example coverage" >> "$GAP_REPORT"
    fi
else
    echo "⚠️  examples/apps directory not found"
    echo "⚠️  **SKIPPED**: No example apps found for coverage analysis" >> "$GAP_REPORT"
fi

echo "→ Output: $EXAMPLE_COVERAGE"

# ============================================================================
# SECTION 5: Documentation vs. Implementation
# ============================================================================
echo "" >> "$GAP_REPORT"
echo "=== SECTION 5: Documentation vs. Implementation ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo "Checking documentation consistency..."

# Check if PROMPT.md exists
if [ -f "PROMPT.md" ]; then
    echo "Found PROMPT.md"
    
    # Extract documented functions from PROMPT.md (look for Module.function pattern)
    grep -oP '\b(IO|List|Map|Enum|String|Path|System)\.[\w_]+' PROMPT.md 2>/dev/null | \
        sed 's/\/[0-9]*$//' | sort -u > /tmp/prompt_funcs.txt
    
    # Extract implemented functions from manifest_stdlib.rs
    grep -oP 'def\s+(public\s+)?[\w_]+\s*\(' src/manifest_stdlib.rs 2>/dev/null | \
        sed 's/def\s*\(public\s*\)\?\([a-zA-Z_][\w_]*\).*/\2/' | sort -u > /tmp/impl_funcs.txt
    
    prompt_count=$(wc -l < /tmp/prompt_funcs.txt)
    impl_count=$(wc -l < /tmp/impl_funcs.txt)
    
    echo "" >> "$GAP_REPORT"
    echo "### Function Documentation Status" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    echo "- **Functions documented in PROMPT.md**: $prompt_count" >> "$GAP_REPORT"
    echo "- **Functions implemented in manifest_stdlib.rs**: $impl_count" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    
    # Find documented but not implemented
    comm -23 /tmp/prompt_funcs.txt /tmp/impl_funcs.txt > /tmp/doc_not_impl.txt
    
    if [ -s /tmp/doc_not_impl.txt ]; then
        echo "⚠️  **Functions documented but not implemented**:" >> "$DOC_MISMATCH"
        echo "" >> "$DOC_MISMATCH"
        cat /tmp/doc_not_impl.txt >> "$DOC_MISMATCH"
        echo "" >> "$DOC_MISMATCH"
        
        echo "⚠️  $(wc -l < /tmp/doc_not_impl.txt) functions documented in PROMPT.md but not found in implementation" >> "$GAP_REPORT"
        echo "→ See $DOC_MISMATCH for details" >> "$GAP_REPORT"
    else
        echo "✅ All documented functions appear to be implemented" >> "$GAP_REPORT"
    fi
else
    echo "⚠️  PROMPT.md not found"
    echo "⚠️  **SKIPPED**: PROMPT.md not found for documentation comparison" >> "$GAP_REPORT"
fi

echo "→ Output: $DOC_MISMATCH"

# ============================================================================
# SECTION 6: Summary and Recommendations
# ============================================================================
echo "" >> "$GAP_REPORT"
echo "=== SECTION 6: Summary and Recommendations ===" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

# Count critical issues
critical_issues=0

if [ -s "$MISSING_HOST_FUNCS" ]; then
    critical_issues=$((critical_issues + 1))
fi

if [ ! -f "src/c_backend/stubs.rs" ]; then
    critical_issues=$((critical_issues + 1))
fi

echo "### Critical Issues Found: $critical_issues" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

if [ "$critical_issues" -gt 0 ]; then
    echo "#### Priority Actions:" >> "$GAP_REPORT"
    echo "" >> "$GAP_REPORT"
    
    if [ -s "$MISSING_HOST_FUNCS" ]; then
        echo "1. **HIGH**: Register missing host functions in src/interop/*.rs" >> "$GAP_REPORT"
        echo "   - $(wc -l < "$MISSING_HOST_FUNCS") functions need registration" >> "$GAP_REPORT"
        echo "" >> "$GAP_REPORT"
    fi
    
    if [ ! -f "src/c_backend/stubs.rs" ]; then
        echo "2. **CRITICAL**: Implement native backend host call handlers" >> "$GAP_REPORT"
        echo "   - src/c_backend/stubs.rs is missing" >> "$GAP_REPORT"
        echo "" >> "$GAP_REPORT"
    fi
else
    echo "✅ No critical issues found - implementation appears complete" >> "$GAP_REPORT"
fi

echo "" >> "$GAP_REPORT"
echo "### Generated Reports:" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"
echo "- $MISSING_HOST_FUNCS - List of unregistered host functions" >> "$GAP_REPORT"
echo "- $NATIVE_PARITY - Native backend parity matrix" >> "$GAP_REPORT"
echo "- $ERROR_MESSAGES - Error message quality audit" >> "$GAP_REPORT"
echo "- $EXAMPLE_COVERAGE - Example app coverage report" >> "$GAP_REPORT"
echo "- $DOC_MISMATCH - Documentation vs. implementation gaps" >> "$GAP_REPORT"
echo "" >> "$GAP_REPORT"

echo ""
echo "========================================"
echo "Discovery Complete!"
echo "========================================"
echo ""
echo "Generated reports:"
echo "  - $GAP_REPORT (main report)"
echo "  - $MISSING_HOST_FUNCS"
echo "  - $NATIVE_PARITY"
echo "  - $ERROR_MESSAGES"
echo "  - $EXAMPLE_COVERAGE"
echo "  - $DOC_MISMATCH"
echo ""
echo "Next steps:"
echo "  1. Review $GAP_REPORT for critical issues"
echo "  2. Address missing host function registrations"
echo "  3. Implement native backend parity"
echo "  4. Improve error messages"
echo "  5. Add missing example apps"
echo ""

# Cleanup
rm -f /tmp/all-host-calls.txt /tmp/registered-funcs.txt /tmp/prompt_funcs.txt /tmp/impl_funcs.txt /tmp/test_output.log /tmp/doc_not_impl.txt

exit 0