# CodeGraph Improvements

## Overview

After testing the entity relationship features (`defines`, `uses`, `deps`, `dependents`), several usability issues were identified that make the tool harder to use than necessary.

---

## 1. Entity Search Requires Exact Signature Matching

### Problem
The `defines` and `uses` commands require an exact signature match including line numbers:

```bash
# ❌ Won't find anything
codegraph defines class ConnectionPool

# ✅ Must include line numbers
codegraph defines class "ConnectionPool (~L40-58)"
```

This is unintuitive. Users don't know the line numbers, and the LLM returns signatures with line info that doesn't match cleanly.

### Root Cause
- The LLM prompt extracts entities in format: `EntityName (~L10-20): description`
- Search uses `LIKE 'EntityName%'` which matches the full signature
- But `ConnectionPool` doesn't match `ConnectionPool (~L40-58)`

### Suggested Fixes

**Option A: Name-only matching**
Parse just the entity name from the signature before searching:
```sql
-- Extract name before the '(' or ':' for matching
WHERE LOWER(SUBSTR(t.signature, 1, INSTR(t.signature, '(') - 1)) = LOWER(?1)
```

**Option B: Flexible signature parsing**
Store both `entity_name` and `entity_signature` columns, match on name.

**Option C: Fuzzy/partial matching**
Accept partial names and match any signature starting with that name.

---

## 2. Short Repository IDs Don't Work

### Problem
```bash
# ❌ Not found
codegraph defines -r 6fb99013 class UserService

# ✅ Full UUID required
codegraph defines -r 6fb99013-7b19-4148-beb7-7d135a7675f8 class UserService
```

### Root Cause
SQL query uses exact match: `AND f.knowledge_id = '6fb99013'` but stored IDs are full UUIDs.

### Suggested Fix
Accept partial IDs and match by prefix:
```rust
// Match by prefix instead of exact match
let knowledge_filter = match knowledge_id {
    Some(kid) => format!("AND f.knowledge_id LIKE '{}%'", kid),
    None => String::new(),
};
```

---

## 3. classes_used Extraction from LLM is Poor

### Problem
Python imports like `from fetchers.base_fetcher import BaseFetcher` aren't captured as `classes_used`.

Test file imports `BaseFetcher` but `codegraph uses BaseFetcher` returns nothing.

### Root Cause
The LLM prompt asks for:
```
classesUsed: string[] — Structural/type definitions from OTHER files that this file instantiates, extends, or uses.
```

But:
1. Python `import X` and `from X import Y` aren't being parsed correctly
2. The LLM doesn't reliably extract imports as "used classes"

### Suggested Fixes

**Option A: Regex-based import extraction**
Don't rely on LLM for imports. Use direct regex parsing:
```python
# Extract Python imports
import re
imports = re.findall(r'from\s+(\S+)\s+import\s+(.+)', content)
```

**Option B: Improve LLM prompt**
Make the prompt more explicit about import formats for each language:
```
For Python files, extract imports from:
- `import module` → modules_used
- `from module import X` → classes_used includes X, modules_used includes module
```

**Option C: Multi-pass extraction**
First pass: regex extract all imports
Second pass: LLM describe how the imports are used

---

## 4. File Path Display vs Actual Search

### Problem
Files like `AGENTS.md` contain TypeScript interface definitions (`CardProps`, etc.) but searching for `CardProps` across all repos finds them in `AGENTS.md` which is confusing.

### Root Cause
`.md` files are being analyzed and can contain code-like entities.

### Suggested Fixes

**Option A: Filter by language**
```bash
codegraph defines -r <id> --language python class CardProps
```

**Option B: Exclude documentation files from entity analysis**
Add `.md` files to skip list for entity extraction but keep for content search.

---

## 5. No "Where is X defined OR used" Combined View

### Problem
Users want to know both where something is defined AND where it's used in one command.

### Current Behavior
```bash
# Two separate commands
codegraph defines class ConnectionPool
codegraph uses class ConnectionPool
```

### Suggested Fix
Add `--all` flag:
```bash
codegraph defines --all class ConnectionPool
# Shows both definition and all usages in one output
```

---

## 6. Dependents Command Limited by Extraction Quality

### Problem
`codegraph dependents -f base_fetcher.py` returns nothing because `classes_used` isn't being populated correctly from imports.

### Root Cause
Dependents finds files that reference entities defined in the target file. If imports aren't extracted, no dependents are found.

### Suggested Fix
See section 3 - fixing `classes_used` extraction will fix this.

---

## 7. Command-line Convenience

### Problem
- `--repo` flag name is verbose
- No way to set default repo for session
- `--repo` vs positional repo ID inconsistency

### Suggested Fixes
```bash
# Short flag
-r, --repo

# Or use current repo context
codegraph defines class ConnectionPool  # uses "current" repo if set
codegraph set-context <repo-id>
```

---

## Priority Recommendations

### High Priority (Blocking usability)
1. **Fix entity name matching** (section 1) - Currently search is broken for most entities
2. **Fix short ID matching** (section 2) - Makes repo filtering unusable
3. **Fix classes_used extraction** (section 3) - Core feature is missing data

### Medium Priority (Usability improvements)
4. Combined defines/uses view (section 5)
5. Dependents fix after classes_used is fixed

### Low Priority (Nice to have)
6. File path language filtering
7. Command-line convenience improvements

---

## Testing Checklist

After implementing fixes, verify:

```bash
# 1. Entity search without line numbers
codegraph defines class ConnectionPool
# Expected: Found in base_fetcher.py

# 2. Short repo ID
codegraph defines -r 6fb99013 class ConnectionPool
# Expected: Works with partial ID

# 3. Uses/imports extraction
codegraph uses class BaseFetcher
# Expected: Shows test files that import it

# 4. Dependents
codegraph dependents -f base_fetcher.py
# Expected: Shows test_websocket_fetcher.py, test_base_fetcher.py

# 5. All-in-one search
codegraph defines --all class ConnectionPool
# Expected: Shows definition + usages
```
