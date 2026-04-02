#!/usr/bin/env bash
# fm.sh — Shared frontmatter extraction and validation CLI for lb-dev.
# Usage:
#   fm.sh get <file> <field>          Extract a single field value
#   fm.sh set <file> <field> <value>  Update a single field value
#   fm.sh validate <file>             Validate frontmatter (exit 0/1)
#   fm.sh validate --stdin            Validate from stdin content
#   fm.sh batch <dir> <field>         Batch extract from tracking files
#   fm.sh scan <dir> [flags]          Scan for multi-field frontmatter (TSV)
#   fm.sh scan-files <dir> [flags]    Scan flat .md files for frontmatter (TSV)
set -euo pipefail

# --- Core functions ---

# Extract raw frontmatter lines from a file (between --- markers)
extract_frontmatter_file() {
  local file="$1"
  [[ -f "$file" ]] || { echo "fm: file not found: $file" >&2; return 1; }
  local first_line
  first_line=$(head -1 "$file")
  [[ "$first_line" == "---" ]] || { echo "fm: no frontmatter in $file" >&2; return 1; }
  sed -n '1d; /^---$/q; p' "$file"
}

extract_frontmatter_stdin() {
  local content
  content=$(cat)
  local first_line
  first_line=$(printf '%s' "$content" | head -1)
  [[ "$first_line" == "---" ]] || { echo "fm: no frontmatter in stdin content" >&2; return 1; }
  printf '%s\n' "$content" | sed -n '1d; /^---$/q; p'
}

get_field() {
  local field="$1"
  local value
  value=$(grep -m1 "^${field}:" | sed "s/^${field}: *//" | sed 's/^"//;s/"$//' | sed "s/^'//;s/'$//")
  if [[ -z "$value" ]]; then
    echo "fm: field '$field' not found" >&2
    return 1
  fi
  printf '%s\n' "$value"
}

set_frontmatter_field() {
  local file="$1"
  local field="$2"
  local value="$3"

  [[ -f "$file" ]] || { echo "fm: file not found: $file" >&2; return 1; }
  [[ "$field" =~ ^[A-Za-z_][A-Za-z0-9_-]*$ ]] || {
    echo "fm: invalid field name: $field" >&2
    return 1
  }
  [[ "$value" != *$'\n'* ]] || {
    echo "fm: multiline values are not supported by fm.sh set" >&2
    return 1
  }

  local tmp
  tmp=$(mktemp "${TMPDIR:-/tmp}/fm-set.XXXXXX") || return 1

  if ! awk -v field="$field" -v value="$value" '
    BEGIN {
      in_frontmatter = 0
      updated = 0
      saw_close = 0
    }
    NR == 1 {
      if ($0 != "---") {
        print "fm: no frontmatter in " FILENAME > "/dev/stderr"
        exit 2
      }
      print
      in_frontmatter = 1
      next
    }
    {
      if (in_frontmatter == 1) {
        if ($0 == "---") {
          if (updated == 0) {
            print field ": " value
            updated = 1
          }
          print
          saw_close = 1
          in_frontmatter = 2
          next
        }
        if ($0 ~ "^" field ":") {
          if (updated == 0) {
            print field ": " value
            updated = 1
          }
          next
        }
      }
      print
    }
    END {
      if (saw_close == 0) {
        print "fm: unterminated frontmatter in " FILENAME > "/dev/stderr"
        exit 2
      }
    }
  ' "$file" > "$tmp"; then
    rm -f "$tmp"
    return 1
  fi

  mv "$tmp" "$file"
}

# --- Subcommands ---

cmd_get() {
  local file="${1:-}"
  local field="${2:-}"
  [[ -n "$file" && -n "$field" ]] || { echo "Usage: fm.sh get <file> <field>" >&2; exit 1; }
  local fm
  fm=$(extract_frontmatter_file "$file") || exit 1
  printf '%s\n' "$fm" | get_field "$field"
}

cmd_set() {
  [[ $# -ge 3 ]] || { echo "Usage: fm.sh set <file> <field> <value>" >&2; exit 1; }
  local file="$1"
  local field="$2"
  shift 2
  local value="$*"
  set_frontmatter_field "$file" "$field" "$value"
}

cmd_validate() {
  local source="file"
  local file=""
  local fm=""

  if [[ "${1:-}" == "--stdin" ]]; then
    source="stdin"
    fm=$(extract_frontmatter_stdin) || exit 1
  else
    file="${1:-}"
    [[ -n "$file" ]] || { echo "Usage: fm.sh validate <file> | fm.sh validate --stdin" >&2; exit 1; }
    fm=$(extract_frontmatter_file "$file") || exit 1
  fi

  local errors=()
  local label="${file:-stdin}"

  # Required fields
  local required=(title doc_type brief confidence created updated revision)
  for f in "${required[@]}"; do
    if ! printf '%s\n' "$fm" | grep -q "^${f}:"; then
      errors+=("Missing required field: $f")
    fi
  done

  # Enum: doc_type
  local doc_type
  doc_type=$(printf '%s\n' "$fm" | grep -m1 '^doc_type:' | sed 's/^doc_type: *//' | tr -d '"'"'" || echo "")
  case "$doc_type" in
    rule|sop|proc|lesson|module|reference|review|finding) ;;
    *) [[ -n "$doc_type" ]] && errors+=("Invalid doc_type '$doc_type'. Must be: rule|sop|proc|lesson|module|reference|review|finding") ;;
  esac

  # Enum: confidence
  local confidence
  confidence=$(printf '%s\n' "$fm" | grep -m1 '^confidence:' | sed 's/^confidence: *//' | tr -d '"'"'" || echo "")
  case "$confidence" in
    speculative|verified|authoritative) ;;
    *) [[ -n "$confidence" ]] && errors+=("Invalid confidence '$confidence'. Must be: speculative|verified|authoritative") ;;
  esac

  # Date format: created, updated
  for df in created updated; do
    local val
    val=$(printf '%s\n' "$fm" | grep -m1 "^${df}:" | sed "s/^${df}: *//" | tr -d '"'"'" || echo "")
    if [[ -n "$val" ]] && ! [[ "$val" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
      errors+=("Field '$df' must be YYYY-MM-DD, got: $val")
    fi
  done

  # Revision: positive integer
  local rev
  rev=$(printf '%s\n' "$fm" | grep -m1 '^revision:' | sed 's/^revision: *//' | tr -d '"'"'" || echo "")
  if [[ -n "$rev" ]] && ! [[ "$rev" =~ ^[1-9][0-9]*$ ]]; then
    errors+=("Field 'revision' must be a positive integer, got: $rev")
  fi

  if [[ ${#errors[@]} -gt 0 ]]; then
    echo "fm: validation failed for $label:" >&2
    for e in "${errors[@]}"; do
      echo "  - $e" >&2
    done
    return 1
  fi
  return 0
}

cmd_batch() {
  local dir="${1:-}"
  local field="${2:-}"
  [[ -n "$dir" && -n "$field" ]] || { echo "Usage: fm.sh batch <dir> <field>" >&2; exit 1; }
  [[ -d "$dir" ]] || { echo "fm: directory not found: $dir" >&2; exit 1; }

  local found=0
  for subdir in "$dir"/*/; do
    [[ -d "$subdir" ]] || continue
    local basename
    basename=$(basename "$subdir")
    local tracking_file=""
    for candidate in session-log.md progress.md tdd.md; do
      if [[ -f "$subdir$candidate" ]]; then
        tracking_file="$subdir$candidate"
        break
      fi
    done
    [[ -n "$tracking_file" ]] || continue
    local value
    value=$(extract_frontmatter_file "$tracking_file" 2>/dev/null | get_field "$field" 2>/dev/null) || {
      echo "fm: warn: could not extract '$field' from $tracking_file" >&2
      continue
    }
    echo "${basename}:${value}"
    found=1
  done

  [[ "$found" -eq 1 ]] || { echo "fm: no tracking files found in $dir" >&2; return 1; }
  return 0
}

# --- Scan subcommand ---

cmd_scan() {
  local dir=""
  local fields="status,brief,updated"
  local pattern="*"
  local progress=0
  local header=0

  # Parse arguments
  while [[ $# -gt 0 ]]; do
    case "$1" in
      --fields)  fields="${2:-}"; shift 2 ;;
      --pattern) pattern="${2:-}"; shift 2 ;;
      --progress) progress=1; shift ;;
      --header)  header=1; shift ;;
      -*)        echo "fm scan: unknown flag '$1'" >&2; exit 1 ;;
      *)
        if [[ -z "$dir" ]]; then
          dir="$1"; shift
        else
          echo "fm scan: unexpected argument '$1'" >&2; exit 1
        fi
        ;;
    esac
  done

  [[ -n "$dir" ]] || { echo "Usage: fm.sh scan <dir> [--fields f1,f2] [--pattern glob] [--progress] [--header]" >&2; exit 1; }
  [[ -d "$dir" ]] || { echo "fm scan: directory not found: $dir" >&2; exit 1; }

  # Split fields into array
  IFS=',' read -ra field_arr <<< "$fields"

  # Header row
  if [[ "$header" -eq 1 ]]; then
    local hdr="dirname"
    for f in "${field_arr[@]}"; do
      hdr+="\t$f"
    done
    [[ "$progress" -eq 1 ]] && hdr+="\tprogress"
    printf '%b\n' "$hdr"
  fi

  local found=0
  for subdir in "$dir"/$pattern/; do
    [[ -d "$subdir" ]] || continue
    local name
    name=$(basename "$subdir")

    # Skip intake-* in procs
    case "$dir" in
      *procs*) case "$name" in intake-*) continue ;; esac ;;
    esac

    # Detect tracking file
    local tracking=""
    for candidate in session-log.md progress.md tdd.md; do
      if [[ -f "$subdir$candidate" ]]; then
        tracking="$subdir$candidate"
        break
      fi
    done
    [[ -n "$tracking" ]] || continue

    # Extract frontmatter once
    local fm
    fm=$(extract_frontmatter_file "$tracking" 2>/dev/null) || continue

    # Build output line
    local line="$name"
    for f in "${field_arr[@]}"; do
      local val
      val=$(printf '%s\n' "$fm" | get_field "$f" 2>/dev/null) || val="—"
      line+="\t$val"
    done

    # Progress column
    if [[ "$progress" -eq 1 ]]; then
      local done_count total_count
      done_count=$(grep -c '\[x\]' "$tracking" 2>/dev/null) || done_count=0
      total_count=$(grep -cE '\[.\]' "$tracking" 2>/dev/null) || total_count=0
      if [[ "$total_count" -gt 0 ]]; then
        local pct=$(( done_count * 100 / total_count ))
        line+="\t${done_count}/${total_count} (${pct}%)"
      else
        line+="\t—"
      fi
    fi

    printf '%b\n' "$line"
    found=1
  done

  [[ "$found" -eq 1 ]] || { echo "fm scan: no tracking files found in $dir" >&2; return 1; }
  return 0
}

# --- Scan-files subcommand ---

cmd_scan_files() {
  local dir=""
  local fields="brief,confidence,updated"
  local exclude="_template.md,index.md"
  local header=0

  while [[ $# -gt 0 ]]; do
    case "$1" in
      --fields)  fields="${2:-}"; shift 2 ;;
      --exclude) exclude="${2:-}"; shift 2 ;;
      --header)  header=1; shift ;;
      -*)        echo "fm scan-files: unknown flag '$1'" >&2; exit 1 ;;
      *)
        if [[ -z "$dir" ]]; then
          dir="$1"; shift
        else
          echo "fm scan-files: unexpected argument '$1'" >&2; exit 1
        fi
        ;;
    esac
  done

  [[ -n "$dir" ]] || { echo "Usage: fm.sh scan-files <dir> [--fields f1,f2] [--exclude pattern] [--header]" >&2; exit 1; }
  [[ -d "$dir" ]] || { echo "fm scan-files: directory not found: $dir" >&2; exit 1; }

  IFS=',' read -ra field_arr <<< "$fields"
  IFS=',' read -ra exclude_arr <<< "$exclude"

  # Header row
  if [[ "$header" -eq 1 ]]; then
    local hdr="file"
    for f in "${field_arr[@]}"; do
      hdr+="\t$f"
    done
    printf '%b\n' "$hdr"
  fi

  local found=0
  for filepath in "$dir"/*.md; do
    [[ -f "$filepath" ]] || continue
    local filename
    filename=$(basename "$filepath")

    # Check exclusion list
    local skip=0
    for ex in "${exclude_arr[@]}"; do
      if [[ "$filename" == "$ex" ]]; then
        skip=1; break
      fi
    done
    [[ "$skip" -eq 1 ]] && continue

    # Extract frontmatter
    local fm
    fm=$(extract_frontmatter_file "$filepath" 2>/dev/null) || continue

    # Build output line (filename without .md extension)
    local name="${filename%.md}"
    local line="$name"
    for f in "${field_arr[@]}"; do
      local val
      val=$(printf '%s\n' "$fm" | get_field "$f" 2>/dev/null) || val="—"
      line+="\t$val"
    done

    printf '%b\n' "$line"
    found=1
  done

  [[ "$found" -eq 1 ]] || { echo "fm scan-files: no .md files with frontmatter in $dir" >&2; return 1; }
  return 0
}

# --- Dispatcher ---

case "${1:-}" in
  get)        shift; cmd_get "$@" ;;
  set)        shift; cmd_set "$@" ;;
  validate)   shift; cmd_validate "$@" ;;
  batch)      shift; cmd_batch "$@" ;;
  scan)       shift; cmd_scan "$@" ;;
  scan-files) shift; cmd_scan_files "$@" ;;
  -h|--help|"")
    echo "Usage: fm.sh <subcommand> [args...]"
    echo "  get        <file> <field>   Extract a single field value"
    echo "  set        <file> <field> <value>"
    echo "                              Update a single field value"
    echo "  validate   <file>           Validate frontmatter completeness"
    echo "  validate   --stdin          Validate from stdin"
    echo "  batch      <dir> <field>    Batch extract from tracking files"
    echo "  scan       <dir> [flags]    Scan dir for multi-field frontmatter (TSV)"
    echo "             --fields f1,f2   Fields to extract (default: status,brief,updated)"
    echo "             --pattern glob   Filter subdirs (default: *)"
    echo "             --progress       Append checkbox progress column"
    echo "             --header         Print header row"
    echo "  scan-files <dir> [flags]    Scan flat .md files for frontmatter (TSV)"
    echo "             --fields f1,f2   Fields to extract (default: brief,confidence,updated)"
    echo "             --exclude a,b    Filenames to skip (default: _template.md,index.md)"
    echo "             --header         Print header row"
    exit 0
    ;;
  *)
    echo "fm: unknown subcommand '$1'. Run fm.sh --help for usage." >&2
    exit 1
    ;;
esac
