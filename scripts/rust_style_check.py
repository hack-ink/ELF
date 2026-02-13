#!/usr/bin/env python3

import argparse
import re
import subprocess
import sys
from dataclasses import dataclass
from pathlib import Path


SERDE_DEFAULT_RE = re.compile(r"^\s*#\s*\[\s*serde\s*\(\s*default\b[^)]*\)\s*]\s*$")
USE_RE = re.compile(r"^\s*(pub\s+)?use\s+(.+);\s*$")
CFG_TEST_RE = re.compile(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*]\s*$")
FN_START_RE = re.compile(
    r"^\s*(pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+\w+"
)
INLINE_BOUNDS_RE = re.compile(
    r"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:fn|impl|struct|enum|trait)\b[^\n{;]*<[^>{}]*\b(?:[A-Za-z_][A-Za-z0-9_]*|'[A-Za-z_][A-Za-z0-9_]*)\s*:(?!:)[^>{}]*>"
)
STD_QUALIFIED_MACRO_RE = re.compile(r"\bstd::(vec|format|println|eprintln|dbg|write|writeln)!\s*\(")
EXPECT_CALL_RE = re.compile(r"\.expect\s*\((.*?)\)")
UNWRAP_CALL_RE = re.compile(r"\.unwrap\s*\(")
NUM_SUFFIX_RE = re.compile(r"\b\d+(?:\.\d+)?(f32|f64|i8|i16|i32|i64|i128|isize|u8|u16|u32|u64|u128|usize)\b")
PLAIN_INT_RE = re.compile(r"\b[1-9]\d{3,}\b")
TEST_ATTR_RE = re.compile(r"^\s*#\s*\[\s*test\s*]\s*$")
SNAKE_CASE_RE = re.compile(r"^[a-z][a-z0-9_]*$")
ASSIGNMENT_STMT_RE = re.compile(
    r"(?:\+=|-=|\*=|/=|%=|&=|\|=|\^=|<<=|>>=|(?<![=!<>])=(?!=))"
)

ITEM_ORDER = {
    "mod": 0,
    "use": 1,
    "macro_rules": 2,
    "type": 3,
    "const": 4,
    "static": 5,
    "trait": 6,
    "enum": 7,
    "struct": 8,
    "impl": 9,
    "fn": 10,
}

STYLE_RULE_IDS = {
    "RUST-STYLE-MOD-001",
    "RUST-STYLE-MOD-002",
    "RUST-STYLE-MOD-003",
    "RUST-STYLE-MOD-005",
    "RUST-STYLE-MOD-007",
    "RUST-STYLE-FILE-001",
    "RUST-STYLE-SERDE-001",
    "RUST-STYLE-IMPORT-001",
    "RUST-STYLE-IMPORT-002",
    "RUST-STYLE-IMPORT-003",
    "RUST-STYLE-IMPORT-004",
    "RUST-STYLE-IMPORT-005",
    "RUST-STYLE-IMPORT-006",
    "RUST-STYLE-IMPORT-007",
    "RUST-STYLE-IMPL-001",
    "RUST-STYLE-IMPL-003",
    "RUST-STYLE-GENERICS-001",
    "RUST-STYLE-LOG-002",
    "RUST-STYLE-RUNTIME-001",
    "RUST-STYLE-RUNTIME-002",
    "RUST-STYLE-NUM-001",
    "RUST-STYLE-NUM-002",
    "RUST-STYLE-READ-002",
    "RUST-STYLE-SPACE-003",
    "RUST-STYLE-SPACE-004",
    "RUST-STYLE-TEST-001",
    "RUST-STYLE-TEST-002",
}
IMPLEMENTED_STYLE_RULE_IDS = {
    "RUST-STYLE-MOD-001",
    "RUST-STYLE-MOD-002",
    "RUST-STYLE-MOD-003",
    "RUST-STYLE-MOD-005",
    "RUST-STYLE-MOD-007",
    "RUST-STYLE-FILE-001",
    "RUST-STYLE-SERDE-001",
    "RUST-STYLE-IMPORT-001",
    "RUST-STYLE-IMPORT-002",
    "RUST-STYLE-IMPORT-003",
    "RUST-STYLE-IMPORT-004",
    "RUST-STYLE-IMPORT-005",
    "RUST-STYLE-IMPORT-006",
    "RUST-STYLE-IMPORT-007",
    "RUST-STYLE-IMPL-001",
    "RUST-STYLE-IMPL-003",
    "RUST-STYLE-GENERICS-001",
    "RUST-STYLE-LOG-002",
    "RUST-STYLE-RUNTIME-001",
    "RUST-STYLE-RUNTIME-002",
    "RUST-STYLE-NUM-001",
    "RUST-STYLE-NUM-002",
    "RUST-STYLE-READ-002",
    "RUST-STYLE-SPACE-003",
    "RUST-STYLE-SPACE-004",
    "RUST-STYLE-TEST-001",
    "RUST-STYLE-TEST-002",
}


@dataclass
class Violation:
    file: Path
    line: int
    rule: str
    message: str

    def format(self) -> str:
        return f"{self.file}:{self.line}:1: [{self.rule}] {self.message}"


@dataclass
class TopItem:
    kind: str
    name: str | None
    line: int
    is_pub: bool
    is_async: bool
    attrs: list[str]
    impl_target: str | None
    raw: str


def git_tracked_rust_files() -> list[Path]:
    result = subprocess.run(
        ["git", "ls-files", "*.rs"],
        check=True,
        text=True,
        capture_output=True,
    )
    return [Path(line) for line in result.stdout.splitlines() if line]


def line_indent_width(line: str) -> int:
    width = 0
    for ch in line:
        if ch == "\t":
            width += 4
        elif ch == " ":
            width += 1
        else:
            break
    return width


def strip_string_and_line_comment_with_state(line: str, in_str: bool) -> tuple[str, bool]:
    out: list[str] = []
    escape = False
    i = 0

    while i < len(line):
        ch = line[i]
        nxt = line[i + 1] if i + 1 < len(line) else ""

        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            out.append(" ")
            i += 1
            continue

        if ch == '"':
            in_str = True
            out.append(" ")
            i += 1
            continue

        if ch == "/" and nxt == "/":
            break

        out.append(ch)
        i += 1

    return "".join(out), in_str


def strip_line_comment_preserve_strings(line: str) -> str:
    out: list[str] = []
    in_str = False
    escape = False
    i = 0

    while i < len(line):
        ch = line[i]
        nxt = line[i + 1] if i + 1 < len(line) else ""

        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            out.append(ch)
            i += 1
            continue

        if ch == '"':
            in_str = True
            out.append(ch)
            i += 1
            continue

        if ch == "/" and nxt == "/":
            break

        out.append(ch)
        i += 1

    return "".join(out)


def strip_string_and_line_comment(line: str) -> str:
    stripped, _ = strip_string_and_line_comment_with_state(line, in_str=False)
    return stripped


def next_non_attribute_line(lines: list[str], idx: int) -> int | None:
    cursor = idx + 1

    while cursor < len(lines):
        stripped = lines[cursor].strip()

        if not stripped:
            cursor += 1
            continue

        if stripped.startswith("#[") or stripped.startswith("///") or stripped.startswith("//!"):
            cursor += 1
            continue

        return cursor

    return None


def extract_use_path(line: str) -> str | None:
    match = USE_RE.match(line)
    if not match:
        return None
    return match.group(2).strip()


def imported_symbols_from_use_path(path: str) -> list[str]:
    compact = path.replace(" ", "")
    if compact.endswith("::*"):
        return []

    def normalize_symbol(segment: str) -> str | None:
        symbol = segment.strip()
        if not symbol:
            return None
        symbol = symbol.split(" as ", 1)[0].strip()
        if symbol in {"*", "self", "super", "crate"}:
            return None
        if "::" in symbol:
            symbol = symbol.rsplit("::", 1)[1]
        if symbol.startswith("r#"):
            symbol = symbol[2:]
        return symbol

    if "{" in path and "}" in path:
        inside = path.split("{", 1)[1].rsplit("}", 1)[0]
        out: list[str] = []
        for segment in inside.split(","):
            symbol = normalize_symbol(segment)
            if symbol:
                out.append(symbol)
        return out

    symbol = normalize_symbol(path.rsplit("::", 1)[-1])
    return [symbol] if symbol else []


def use_origin(path: str) -> int:
    trimmed = path.replace("pub ", "")
    root = trimmed.lstrip(":").split("::", 1)[0]

    if root in {"std", "core", "alloc"}:
        return 0
    if root in {"crate", "self", "super"} or root.startswith("elf_"):
        return 2
    return 1


def is_visibility_pub(line: str) -> bool:
    stripped = line.lstrip()
    return stripped.startswith("pub ") or stripped.startswith("pub(")


def detect_top_item(line: str, attrs: list[str], line_no: int) -> TopItem | None:
    stripped = line.strip()
    if not stripped or stripped.startswith("//"):
        return None

    mod_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*(;|\{)", line)
    if mod_match:
        return TopItem("mod", mod_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    if re.match(r"^\s*(pub\s+)?use\s+", line):
        return TopItem("use", None, line_no, is_visibility_pub(line), False, attrs, None, line)

    if re.match(r"^\s*macro_rules!\s*", line):
        return TopItem("macro_rules", None, line_no, False, False, attrs, None, line)

    type_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?type\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if type_match:
        return TopItem("type", type_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    const_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?const\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if const_match:
        return TopItem("const", const_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    static_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?static\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if static_match:
        return TopItem("static", static_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    trait_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?trait\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if trait_match:
        return TopItem("trait", trait_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    enum_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?enum\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if enum_match:
        return TopItem("enum", enum_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    struct_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?struct\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if struct_match:
        return TopItem("struct", struct_match.group(2), line_no, is_visibility_pub(line), False, attrs, None, line)

    if re.match(r"^\s*impl\b", line):
        impl_target: str | None = None
        after_impl = line.split("impl", 1)[1].strip()
        if " for " in after_impl:
            right = after_impl.split(" for ", 1)[1].strip()
            impl_target = re.split(r"[<{\s]", right, maxsplit=1)[0].split("::")[-1]
        else:
            impl_target = re.split(r"[<{\s]", after_impl, maxsplit=1)[0].split("::")[-1]
        return TopItem("impl", None, line_no, is_visibility_pub(line), False, attrs, impl_target, line)

    fn_match = re.match(r"^\s*(pub(?:\([^)]*\))?\s+)?(async\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+([A-Za-z_][A-Za-z0-9_]*)", line)
    if fn_match:
        return TopItem("fn", fn_match.group(3), line_no, is_visibility_pub(line), fn_match.group(2) is not None, attrs, None, line)

    return None


def parse_top_level_items(lines: list[str]) -> list[TopItem]:
    items: list[TopItem] = []
    attrs: list[str] = []
    depth = 0

    for idx, raw in enumerate(lines):
        line = raw.rstrip("\n")
        stripped = line.strip()

        if depth == 0 and stripped.startswith("#"):
            attrs.append(stripped)

        if depth == 0:
            item = detect_top_item(line, attrs.copy(), idx + 1)
            if item:
                items.append(item)
                attrs.clear()
            elif stripped and not stripped.startswith("#") and not stripped.startswith("//"):
                attrs.clear()

        code = strip_string_and_line_comment(line)
        depth += code.count("{")
        depth -= code.count("}")

        if depth < 0:
            depth = 0

    return items


def find_function_ranges(lines: list[str]) -> list[tuple[int, int]]:
    ranges: list[tuple[int, int]] = []
    pending_fn = False
    brace_depth = 0
    body_start: int | None = None

    for idx, line in enumerate(lines):
        code = strip_string_and_line_comment(line)

        if not pending_fn and brace_depth == 0 and FN_START_RE.search(code):
            if code.rstrip().endswith(";"):
                continue
            pending_fn = True

        if pending_fn and body_start is None:
            open_idx = code.find("{")
            if open_idx != -1:
                body_start = idx
                brace_depth = 1

                segment = code[open_idx + 1 :]
                brace_depth += segment.count("{")
                brace_depth -= segment.count("}")

                if brace_depth == 0:
                    ranges.append((body_start, idx))
                    pending_fn = False
                    body_start = None
                continue

        if body_start is not None:
            brace_depth += code.count("{")
            brace_depth -= code.count("}")

            if brace_depth == 0:
                ranges.append((body_start, idx))
                pending_fn = False
                body_start = None

    return ranges


def first_significant_statement_line(lines: list[str]) -> str | None:
    for line in lines:
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("//") or stripped.startswith("#"):
            continue
        return stripped
    return None


def last_significant_statement_line(lines: list[str]) -> str | None:
    for line in reversed(lines):
        stripped = line.strip()
        if not stripped:
            continue
        if stripped.startswith("//") or stripped.startswith("#"):
            continue
        return stripped
    return None


def normalize_statement_text(statement_lines: list[str]) -> str:
    parts: list[str] = []
    in_str = False
    for raw in statement_lines:
        code, in_str = strip_string_and_line_comment_with_state(raw, in_str)
        code = code.strip()
        if not code:
            continue
        if code.startswith("#"):
            continue
        parts.append(code)
    return " ".join(parts)


def strip_turbofish(text: str) -> str:
    out: list[str] = []
    i = 0

    while i < len(text):
        if text.startswith("::<", i):
            i += 3
            depth = 1
            while i < len(text) and depth > 0:
                ch = text[i]
                if ch == "<":
                    depth += 1
                elif ch == ">":
                    depth -= 1
                i += 1
            continue
        out.append(text[i])
        i += 1

    return "".join(out)


def parse_ufcs_target_call(text: str) -> tuple[str, str] | None:
    if not text.startswith("<"):
        return None

    depth = 0
    close_idx = -1
    for idx, ch in enumerate(text):
        if ch == "<":
            depth += 1
        elif ch == ">":
            depth -= 1
            if depth == 0:
                close_idx = idx
                break

    if close_idx == -1:
        return None

    body = text[1:close_idx].strip()
    rest = text[close_idx + 1 :].lstrip()
    if not rest.startswith("::"):
        return None

    rest = rest[2:]
    fn_match = re.match(r"^(?P<func>[A-Za-z_][A-Za-z0-9_]*)\s*\(", rest)
    if not fn_match:
        return None

    func = fn_match.group("func")
    if " as " in body:
        target = body.split(" as ", 1)[1].strip()
    else:
        target = body

    if not target:
        return None
    return target, func


def classify_statement_type(statement_lines: list[str]) -> str:
    normalized = normalize_statement_text(statement_lines)
    if not normalized:
        return "empty"
    normalized = strip_turbofish(normalized)
    first = normalized

    if re.match(r"^let\b", first):
        return "let"
    if re.match(r"^if\s+let\b", first):
        return "if-let"
    if re.match(r"^if\b", first):
        return "if"
    if re.match(r"^match\b", first):
        return "match"
    if re.match(r"^for\b", first):
        return "for"
    if re.match(r"^while\b", first):
        return "while"
    if re.match(r"^loop\b", first):
        return "loop"
    if re.match(
        r"^[A-Za-z_][A-Za-z0-9_]*(?:\.[A-Za-z_][A-Za-z0-9_]*)*(?:\.await)?\?\s*;?$",
        first,
    ):
        return "try-expr"
    if re.search(ASSIGNMENT_STMT_RE, first):
        return "assign"

    macro_match = re.match(r"^(?P<name>[A-Za-z_][A-Za-z0-9_:]*)!\s*\(", first)
    if macro_match:
        macro_name = macro_match.group("name")
        if "::" in macro_name:
            return "macro-path"
        return "macro"

    ufcs_call = parse_ufcs_target_call(first)
    if ufcs_call:
        return "path-call"

    path_call_match = re.match(
        r"^(?P<target>[A-Za-z_][A-Za-z0-9_]*(?:::[A-Za-z_][A-Za-z0-9_]*)+)\s*\(",
        first,
    )
    if path_call_match:
        return "path-call"

    fn_call_match = re.match(r"^(?P<target>[A-Za-z_][A-Za-z0-9_]*)\s*\(", first)
    if fn_call_match:
        return "call"

    method_match = re.match(r"^[^;]*\.(?P<method>[A-Za-z_][A-Za-z0-9_]*)\s*\(", first)
    if method_match:
        return "method"

    token = re.split(r"[\s({;]", first, maxsplit=1)[0]
    if token:
        return f"shape:{token}"
    return "other"


def extract_top_level_statements(lines: list[str], fn_start: int, fn_end: int) -> list[tuple[int, int, str]]:
    statements: list[tuple[int, int, str]] = []
    brace_depth = 1
    paren_depth = 0
    bracket_depth = 0
    current_start: int | None = None

    for idx in range(fn_start + 1, fn_end):
        raw_line = lines[idx]
        stripped = raw_line.strip()
        code = strip_string_and_line_comment(raw_line)

        if (
            current_start is None
            and brace_depth == 1
            and stripped
            and not stripped.startswith("//")
            and not stripped.startswith("#")
            and stripped != "}"
        ):
            current_start = idx

        for ch in code:
            if ch == "(":
                paren_depth += 1
            elif ch == ")":
                paren_depth = max(paren_depth - 1, 0)
            elif ch == "[":
                bracket_depth += 1
            elif ch == "]":
                bracket_depth = max(bracket_depth - 1, 0)
            elif ch == "{":
                brace_depth += 1
            elif ch == "}":
                brace_depth -= 1
                if brace_depth < 0:
                    brace_depth = 0

        if current_start is None:
            continue

        stripped_code = code.strip()
        statement_closed = (
            brace_depth == 1
            and paren_depth == 0
            and bracket_depth == 0
            and stripped_code != ""
            and (stripped_code.endswith(";") or stripped_code.endswith("}"))
        )

        if statement_closed:
            span_lines = lines[current_start : idx + 1]
            statements.append((current_start, idx, classify_statement_type(span_lines)))
            current_start = None

    if current_start is not None:
        span_lines = lines[current_start:fn_end]
        statements.append((current_start, fn_end - 1, classify_statement_type(span_lines)))

    return statements


def is_return_or_tail_statement(statement_lines: list[str]) -> bool:
    first = first_significant_statement_line(statement_lines)
    if first is None:
        return False
    if re.match(r"^return\b", first):
        return True

    last = last_significant_statement_line(statement_lines)
    if last is None:
        return False
    if re.match(r"^return\b", last):
        return True
    if last.endswith(";"):
        return False
    if last.endswith("{"):
        return False
    if last in {"}", "};"}:
        return False
    return True


def is_explicit_return_statement(statement_lines: list[str]) -> bool:
    first = first_significant_statement_line(statement_lines)
    if first is None:
        return False
    return re.match(r"^return\b", first) is not None


def extract_top_level_brace_blocks_in_span(lines: list[str], span_start: int, span_end: int) -> list[tuple[int, int]]:
    blocks: list[tuple[int, int]] = []
    depth = 0
    current_start: int | None = None

    for idx in range(span_start, span_end + 1):
        code = strip_string_and_line_comment(lines[idx])
        for ch in code:
            if ch == "{":
                depth += 1
                if depth == 1:
                    current_start = idx
            elif ch == "}":
                if depth == 1 and current_start is not None:
                    blocks.append((current_start, idx))
                    current_start = None
                depth = max(depth - 1, 0)

    return blocks


def is_data_like_brace_block(lines: list[str], block_start: int, block_end: int) -> bool:
    content: list[str] = []
    for idx in range(block_start + 1, block_end):
        code = strip_string_and_line_comment(lines[idx]).strip()
        if not code:
            continue
        if code.startswith("#"):
            continue
        content.append(code)

    if not content:
        return True

    for line in content:
        if "=>" in line:
            return False
        if ";" in line:
            return False
        if re.match(r"^(if|if\s+let|match|for|while|loop|return|let)\b", line):
            return False

    for line in content:
        if re.match(r"^[A-Za-z_][A-Za-z0-9_]*\s*:\s*.+,?$", line):
            continue
        if line.endswith(","):
            continue
        return False

    return True


def check_mod_rs(file: Path) -> list[Violation]:
    if file.name == "mod.rs":
        return [
            Violation(
                file=file,
                line=1,
                rule="RUST-STYLE-FILE-001",
                message="Do not use mod.rs. Use flat module files instead.",
            )
        ]
    return []


def check_serde_option_default(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for idx, line in enumerate(lines):
        if not SERDE_DEFAULT_RE.match(line):
            continue

        next_idx = next_non_attribute_line(lines, idx)
        if next_idx is None:
            continue

        if ": Option<" not in lines[next_idx]:
            continue

        violations.append(
            Violation(
                file=file,
                line=idx + 1,
                rule="RUST-STYLE-SERDE-001",
                message="Do not use #[serde(default)] on Option<T> fields.",
            )
        )

    return violations


def check_error_rs_no_use(file: Path, lines: list[str]) -> list[Violation]:
    if file.name != "error.rs":
        return []

    violations: list[Violation] = []
    for idx, line in enumerate(lines, start=1):
        if re.match(r"^\s*use\s+", line):
            violations.append(
                Violation(
                    file=file,
                    line=idx,
                    rule="RUST-STYLE-IMPORT-005",
                    message="Do not add use imports in error.rs; use fully qualified paths.",
                )
            )

    return violations


def check_import_rules(file: Path, lines: list[str], items: list[TopItem]) -> list[Violation]:
    violations: list[Violation] = []

    # Import grouping rules apply to local imports, not public re-exports.
    use_items = [item for item in items if item.kind == "use" and not item.is_pub]
    has_prelude_glob = any(
        (extract_use_path(lines[item.line - 1]) or "").replace(" ", "") == "crate::prelude::*"
        for item in use_items
    )

    for item in use_items:
        line = lines[item.line - 1]
        path = extract_use_path(line)
        if not path:
            continue

        alias_match = re.search(r"\bas\s+([A-Za-z_][A-Za-z0-9_]*)\b", path)
        if alias_match and alias_match.group(1) != "_":
            violations.append(
                Violation(
                    file=file,
                    line=item.line,
                    rule="RUST-STYLE-IMPORT-003",
                    message="Import aliases are not allowed except `as _` in test keep-alive modules.",
                )
            )

        compact_path = path.replace(" ", "")
        if has_prelude_glob and compact_path.startswith("crate::") and compact_path != "crate::prelude::*":
            violations.append(
                Violation(
                    file=file,
                    line=item.line,
                    rule="RUST-STYLE-IMPORT-007",
                    message="Avoid redundant crate imports when crate::prelude::* is imported.",
                )
            )

        if "::" in path:
            imported_symbols = imported_symbols_from_use_path(path)
            for symbol in imported_symbols:
                if not symbol or not symbol[0].islower():
                    continue

                local_fn_def_re = re.compile(
                    rf"^\s*(?:pub(?:\([^)]*\))?\s+)?(?:async\s+)?(?:const\s+)?(?:unsafe\s+)?fn\s+{re.escape(symbol)}\b"
                )
                local_macro_def_re = re.compile(
                    rf"^\s*(?:macro_rules!\s*{re.escape(symbol)}\b|macro\s+{re.escape(symbol)}\b)"
                )
                unqualified_fn_call_re = re.compile(rf"(?<!:)\b{re.escape(symbol)}\s*\(")
                unqualified_macro_call_re = re.compile(rf"(?<!:)\b{re.escape(symbol)}!\s*\(")

                local_fn_defined = any(
                    local_fn_def_re.search(code)
                    for code in (strip_string_and_line_comment(x) for x in lines)
                )
                local_macro_defined = any(
                    local_macro_def_re.search(code)
                    for code in (strip_string_and_line_comment(x) for x in lines)
                )
                called_fn_unqualified = any(
                    unqualified_fn_call_re.search(strip_string_and_line_comment(x)) for x in lines
                )
                called_macro_unqualified = any(
                    unqualified_macro_call_re.search(strip_string_and_line_comment(x)) for x in lines
                )

                if (called_fn_unqualified and not local_fn_defined) or (
                    called_macro_unqualified and not local_macro_defined
                ):
                    violations.append(
                        Violation(
                            file=file,
                            line=item.line,
                            rule="RUST-STYLE-IMPORT-004",
                            message=(
                                "Do not import free functions or macros into scope; "
                                "prefer qualified module paths."
                            ),
                        )
                    )
                    break

    for prev, curr in zip(use_items, use_items[1:]):
        prev_path = extract_use_path(lines[prev.line - 1])
        curr_path = extract_use_path(lines[curr.line - 1])
        if not prev_path or not curr_path:
            continue

        prev_origin = use_origin(prev_path)
        curr_origin = use_origin(curr_path)

        if curr_origin < prev_origin:
            violations.append(
                Violation(
                    file=file,
                    line=curr.line,
                    rule="RUST-STYLE-IMPORT-001",
                    message="Import groups must be ordered: std, third-party, self/workspace.",
                )
            )

        between = lines[prev.line : curr.line - 1]
        has_blank = any(not line.strip() for line in between)
        has_header_comment = any(line.strip().startswith("//") for line in between)

        if curr_origin != prev_origin and not has_blank:
            violations.append(
                Violation(
                    file=file,
                    line=curr.line,
                    rule="RUST-STYLE-IMPORT-002",
                    message="Separate import groups with one blank line.",
                )
            )
        if curr_origin == prev_origin and has_blank:
            violations.append(
                Violation(
                    file=file,
                    line=curr.line,
                    rule="RUST-STYLE-IMPORT-002",
                    message="Do not place blank lines inside an import group.",
                )
            )
        if has_header_comment:
            violations.append(
                Violation(
                    file=file,
                    line=curr.line,
                    rule="RUST-STYLE-IMPORT-002",
                    message="Do not use header comments for import groups.",
                )
            )

    return violations


def check_module_order(file: Path, items: list[TopItem]) -> list[Violation]:
    violations: list[Violation] = []

    def is_cfg_test_mod(item: TopItem) -> bool:
        if item.kind != "mod":
            return False
        return any(CFG_TEST_RE.match(attr) for attr in item.attrs)

    def order_bucket(kind: str) -> int | None:
        # Keep types and impls in one stage so we can enforce per-type adjacency
        # in MOD-005 without conflicting with MOD-001.
        if kind in {"enum", "struct", "impl"}:
            return 8
        return ITEM_ORDER.get(kind)

    items_for_order = [item for item in items if not is_cfg_test_mod(item)]

    order_seen: list[int] = []
    for item in items_for_order:
        order = order_bucket(item.kind)
        if order is None:
            continue
        if order_seen and order < order_seen[-1]:
            violations.append(
                Violation(
                    file=file,
                    line=item.line,
                    rule="RUST-STYLE-MOD-001",
                    message="Top-level module item order does not match rust.md order.",
                )
            )
        order_seen.append(order)

    non_pub_seen: dict[str, bool] = {}
    for item in items_for_order:
        seen_non_pub = non_pub_seen.get(item.kind, False)
        if item.is_pub:
            if seen_non_pub:
                violations.append(
                    Violation(
                        file=file,
                        line=item.line,
                        rule="RUST-STYLE-MOD-002",
                        message="Place pub items before non-pub items within the same group.",
                    )
                )
        else:
            non_pub_seen[item.kind] = True

    async_seen = {True: False, False: False}
    for item in items_for_order:
        if item.kind != "fn":
            continue
        key = item.is_pub
        if item.is_async:
            async_seen[key] = True
        elif async_seen[key]:
            violations.append(
                Violation(
                    file=file,
                    line=item.line,
                    rule="RUST-STYLE-MOD-003",
                    message="Place non-async functions before async functions at the same visibility.",
                )
            )

    last_non_test_index = -1
    for idx, item in enumerate(items):
        if not is_cfg_test_mod(item):
            last_non_test_index = idx
    for idx, item in enumerate(items):
        if not is_cfg_test_mod(item):
            continue
        if idx < last_non_test_index:
            violations.append(
                Violation(
                    file=file,
                    line=item.line,
                    rule="RUST-STYLE-MOD-001",
                    message="Place #[cfg(test)] modules after all non-test items.",
                )
            )

    return violations


def check_cfg_test_mod_tests_use_super(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []
    idx = 0

    while idx < len(lines):
        if not CFG_TEST_RE.match(lines[idx]):
            idx += 1
            continue

        j = idx + 1
        while j < len(lines) and not lines[j].strip():
            j += 1
        if j >= len(lines):
            break

        mod_match = re.match(r"^\s*mod\s+([A-Za-z_][A-Za-z0-9_]*)\s*\{", lines[j])
        if not mod_match:
            idx = j + 1
            continue

        mod_name = mod_match.group(1)
        if mod_name == "_test":
            idx = j + 1
            continue

        depth = 0
        found_super_use = False
        k = j
        while k < len(lines):
            code = strip_string_and_line_comment(lines[k])
            if "use super::*;" in code:
                found_super_use = True
            depth += code.count("{")
            depth -= code.count("}")
            if k > j and depth <= 0:
                break
            k += 1

        if mod_name == "tests" and not found_super_use:
            violations.append(
                Violation(
                    file=file,
                    line=j + 1,
                    rule="RUST-STYLE-MOD-007",
                    message="#[cfg(test)] mod tests should include `use super::*;` unless it is a keep-alive module.",
                )
            )

        idx = k + 1

    return violations


def find_top_level_item_end_line(lines: list[str], start_idx: int) -> int:
    depth = 0
    seen_open = False

    for idx in range(start_idx, len(lines)):
        code = strip_string_and_line_comment(lines[idx])
        stripped = code.strip()

        if not seen_open and "{" in code:
            seen_open = True

        depth += code.count("{")
        depth -= code.count("}")

        if seen_open:
            if depth <= 0:
                return idx
        elif stripped.endswith(";"):
            return idx

    return start_idx


def check_impl_adjacency(file: Path, lines: list[str], items: list[TopItem]) -> list[Violation]:
    violations: list[Violation] = []

    type_indices: dict[str, int] = {}
    for idx, item in enumerate(items):
        if item.kind not in {"struct", "enum"} or not item.name:
            continue
        type_indices[item.name] = idx

    impl_by_target: dict[str, list[int]] = {}
    for idx, item in enumerate(items):
        if item.kind != "impl" or not item.impl_target:
            continue
        impl_by_target.setdefault(item.impl_target, []).append(idx)

    for target, impl_indices in impl_by_target.items():
        first_impl = impl_indices[0]
        last_impl = impl_indices[-1]

        for idx in range(first_impl, last_impl + 1):
            item = items[idx]
            if item.kind != "impl" or item.impl_target != target:
                violations.append(
                    Violation(
                        file=file,
                        line=item.line,
                        rule="RUST-STYLE-IMPL-003",
                        message=f"impl blocks for `{target}` must be contiguous.",
                    )
                )
                break

        order_values = [classify_impl_trait_order(items[idx].raw) for idx in impl_indices]
        for pos, (prev, curr) in enumerate(zip(order_values, order_values[1:]), start=1):
            if curr < prev:
                violations.append(
                    Violation(
                        file=file,
                        line=items[impl_indices[pos]].line,
                        rule="RUST-STYLE-IMPL-003",
                        message=(
                            f"impl block order for `{target}` must be inherent, std traits, "
                            "third-party traits, then workspace-member traits."
                        ),
                    )
                )
                break

    for type_name, type_idx in type_indices.items():
        impl_indices = impl_by_target.get(type_name, [])
        if not impl_indices:
            continue

        first_impl = impl_indices[0]
        if first_impl != type_idx + 1:
            violations.append(
                Violation(
                    file=file,
                    line=items[first_impl].line,
                    rule="RUST-STYLE-MOD-005",
                    message=f"Keep `{type_name}` definitions and related impl blocks adjacent.",
                )
            )
            continue

        type_end = find_top_level_item_end_line(lines, items[type_idx].line - 1)
        impl_start = items[first_impl].line - 1
        between = lines[type_end + 1 : impl_start]
        if any(not line.strip() for line in between):
            violations.append(
                Violation(
                    file=file,
                    line=items[first_impl].line,
                    rule="RUST-STYLE-MOD-005",
                    message=(
                        f"Do not insert blank lines between `{type_name}` and its first impl block."
                    ),
                )
            )

    return violations


def classify_impl_trait_order(raw: str) -> int:
    header = strip_string_and_line_comment(raw)
    if " for " not in header:
        return 0

    left = header.split(" for ", 1)[0]
    trait_part = left.split("impl", 1)[1].strip()
    if trait_part.startswith("<") and ">" in trait_part:
        trait_part = trait_part.split(">", 1)[1].strip()
    trait_name = re.split(r"[<\s{]", trait_part, maxsplit=1)[0]

    if trait_name.startswith(("std::", "core::", "alloc::")):
        return 1
    if trait_name.startswith(("crate::", "self::", "super::", "elf_")):
        return 3
    return 2


def find_impl_block_end(lines: list[str], start_idx: int) -> int:
    depth = 0
    seen_open = False

    for idx in range(start_idx, len(lines)):
        code = strip_string_and_line_comment(lines[idx])
        if not seen_open and "{" in code:
            seen_open = True
        depth += code.count("{")
        depth -= code.count("}")
        if seen_open and depth <= 0:
            return idx

    return len(lines) - 1


def find_matching_paren(source: str, open_idx: int) -> int | None:
    depth = 0
    in_str = False
    escape = False
    in_char = False
    char_escape = False
    in_line_comment = False
    block_comment_depth = 0
    i = open_idx

    while i < len(source):
        ch = source[i]
        nxt = source[i + 1] if i + 1 < len(source) else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            i += 1
            continue

        if block_comment_depth > 0:
            if ch == "/" and nxt == "*":
                block_comment_depth += 1
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_comment_depth -= 1
                i += 2
                continue
            i += 1
            continue

        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            i += 1
            continue

        if in_char:
            if char_escape:
                char_escape = False
            elif ch == "\\":
                char_escape = True
            elif ch == "'":
                in_char = False
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue

        if ch == "/" and nxt == "*":
            block_comment_depth += 1
            i += 2
            continue

        if ch == '"':
            in_str = True
            escape = False
            i += 1
            continue

        if ch == "'":
            in_char = True
            char_escape = False
            i += 1
            continue

        if ch == "(":
            depth += 1
        elif ch == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1

    return None


def extract_tracing_macro_calls(lines: list[str]) -> list[tuple[int, str]]:
    source = "\n".join(lines)
    macro_prefixes = (
        "tracing::trace",
        "tracing::debug",
        "tracing::info",
        "tracing::warn",
        "tracing::error",
    )
    calls: list[tuple[int, str]] = []
    i = 0
    line_no = 1
    in_str = False
    escape = False
    in_char = False
    char_escape = False
    in_line_comment = False
    block_comment_depth = 0

    while i < len(source):
        ch = source[i]
        nxt = source[i + 1] if i + 1 < len(source) else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
                line_no += 1
            i += 1
            continue

        if block_comment_depth > 0:
            if ch == "/" and nxt == "*":
                block_comment_depth += 1
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_comment_depth -= 1
                i += 2
                continue
            if ch == "\n":
                line_no += 1
            i += 1
            continue

        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            if ch == "\n":
                line_no += 1
            i += 1
            continue

        if in_char:
            if char_escape:
                char_escape = False
            elif ch == "\\":
                char_escape = True
            elif ch == "'":
                in_char = False
            if ch == "\n":
                line_no += 1
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue

        if ch == "/" and nxt == "*":
            block_comment_depth += 1
            i += 2
            continue

        if ch == '"':
            in_str = True
            escape = False
            i += 1
            continue

        if ch == "'":
            in_char = True
            char_escape = False
            i += 1
            continue

        matched_prefix: str | None = None
        for prefix in macro_prefixes:
            if source.startswith(prefix, i):
                prev = source[i - 1] if i > 0 else ""
                if not (prev.isalnum() or prev == "_"):
                    matched_prefix = prefix
                break

        if matched_prefix:
            start_line = line_no
            cursor = i + len(matched_prefix)
            while cursor < len(source) and source[cursor].isspace():
                cursor += 1
            if cursor >= len(source) or source[cursor] != "!":
                if ch == "\n":
                    line_no += 1
                i += 1
                continue

            cursor += 1
            while cursor < len(source) and source[cursor].isspace():
                cursor += 1
            if cursor >= len(source) or source[cursor] != "(":
                if ch == "\n":
                    line_no += 1
                i += 1
                continue

            end_paren = find_matching_paren(source, cursor)
            if end_paren is None:
                if ch == "\n":
                    line_no += 1
                i += 1
                continue

            args = source[cursor + 1 : end_paren]
            calls.append((start_line, args))
            line_no += source[i : end_paren + 1].count("\n")
            i = end_paren + 1
            continue

        if ch == "\n":
            line_no += 1
        i += 1

    return calls


def split_top_level_args(args: str) -> list[str]:
    parts: list[str] = []
    start = 0
    paren = 0
    brace = 0
    bracket = 0
    in_str = False
    escape = False
    in_char = False
    char_escape = False
    in_line_comment = False
    block_comment_depth = 0
    i = 0

    while i < len(args):
        ch = args[i]
        nxt = args[i + 1] if i + 1 < len(args) else ""

        if in_line_comment:
            if ch == "\n":
                in_line_comment = False
            i += 1
            continue

        if block_comment_depth > 0:
            if ch == "/" and nxt == "*":
                block_comment_depth += 1
                i += 2
                continue
            if ch == "*" and nxt == "/":
                block_comment_depth -= 1
                i += 2
                continue
            i += 1
            continue

        if in_str:
            if escape:
                escape = False
            elif ch == "\\":
                escape = True
            elif ch == '"':
                in_str = False
            i += 1
            continue

        if in_char:
            if char_escape:
                char_escape = False
            elif ch == "\\":
                char_escape = True
            elif ch == "'":
                in_char = False
            i += 1
            continue

        if ch == "/" and nxt == "/":
            in_line_comment = True
            i += 2
            continue

        if ch == "/" and nxt == "*":
            block_comment_depth += 1
            i += 2
            continue

        if ch == '"':
            in_str = True
            escape = False
            i += 1
            continue

        if ch == "'":
            in_char = True
            char_escape = False
            i += 1
            continue

        if ch == "(":
            paren += 1
        elif ch == ")":
            paren = max(paren - 1, 0)
        elif ch == "{":
            brace += 1
        elif ch == "}":
            brace = max(brace - 1, 0)
        elif ch == "[":
            bracket += 1
        elif ch == "]":
            bracket = max(bracket - 1, 0)
        elif ch == "," and paren == 0 and brace == 0 and bracket == 0:
            segment = args[start:i].strip()
            if segment:
                parts.append(segment)
            start = i + 1

        i += 1

    tail = args[start:].strip()
    if tail:
        parts.append(tail)
    return parts


def parse_string_literal(text: str) -> str | None:
    stripped = text.strip()
    if len(stripped) >= 2 and stripped[0] == '"' and stripped[-1] == '"':
        return stripped[1:-1]

    raw_match = re.match(r'^r(?P<hashes>#+)?"(?P<body>[\s\S]*)"(?P=hashes)?$', stripped)
    if raw_match:
        return raw_match.group("body")

    return None


def is_sentence(text: str) -> bool:
    normalized = " ".join(text.split())
    if not normalized:
        return False
    return normalized[0].isupper() and normalized[-1] in {".", "!", "?"}


def has_structured_fields(text: str) -> bool:
    return bool(
        re.search(r"\b[A-Za-z_][A-Za-z0-9_]*\s*=", text)
        or re.search(r"[%?]\s*[A-Za-z_][A-Za-z0-9_:]*", text)
    )


def check_impl_rules(file: Path, lines: list[str], items: list[TopItem]) -> list[Violation]:
    violations: list[Violation] = []

    impl_by_target: dict[str, list[TopItem]] = {}
    for item in items:
        if item.kind != "impl" or not item.impl_target:
            continue
        impl_by_target.setdefault(item.impl_target, []).append(item)

    for target, impls in impl_by_target.items():
        qualified_target = (
            rf"(?:{re.escape(target)}\b|(?:crate|self|super)::(?:[A-Za-z_][A-Za-z0-9_]*::)*{re.escape(target)}\b)"
        )
        return_self_type_re = re.compile(rf"->\s*{qualified_target}")
        param_self_type_re = re.compile(rf"(?<!:):\s*{qualified_target}")

        for item in impls:
            start = item.line - 1
            end = find_impl_block_end(lines, start)
            for idx in range(start, end + 1):
                code = strip_string_and_line_comment(lines[idx]).strip()
                if "fn " not in code:
                    continue
                if return_self_type_re.search(code):
                    violations.append(
                        Violation(
                            file=file,
                            line=idx + 1,
                            rule="RUST-STYLE-IMPL-001",
                            message=f"Use Self instead of concrete type `{target}` in impl method signatures.",
                        )
                    )
                if param_self_type_re.search(code):
                    violations.append(
                        Violation(
                            file=file,
                            line=idx + 1,
                            rule="RUST-STYLE-IMPL-001",
                            message=f"Use Self instead of concrete type `{target}` in impl method signatures.",
                        )
                    )

    return violations


def check_inline_trait_bounds(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for idx, line in enumerate(lines, start=1):
        code = strip_string_and_line_comment(line)
        if INLINE_BOUNDS_RE.match(code):
            violations.append(
                Violation(
                    file=file,
                    line=idx,
                    rule="RUST-STYLE-GENERICS-001",
                    message="Inline trait bounds are not allowed. Move bounds into a where clause.",
                )
            )

    return violations


def check_std_macro_calls(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for idx, line in enumerate(lines, start=1):
        code = strip_string_and_line_comment(line)

        if STD_QUALIFIED_MACRO_RE.search(code):
            violations.append(
                Violation(
                    file=file,
                    line=idx,
                    rule="RUST-STYLE-IMPORT-006",
                    message="Do not qualify standard macros with std::.",
                )
            )

    return violations


def check_logging_quality(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for line_no, args in extract_tracing_macro_calls(lines):
        parts = split_top_level_args(args)
        if not parts:
            continue

        message = parse_string_literal(parts[-1])
        head_parts = parts[:-1] if message is not None else parts
        head_text = ", ".join(head_parts)

        if message is not None:
            if "{" in message or "}" in message:
                violations.append(
                    Violation(
                        file=file,
                        line=line_no,
                        rule="RUST-STYLE-LOG-002",
                        message="Do not interpolate dynamic values in log message strings; use structured fields.",
                    )
                )
            if not is_sentence(message):
                violations.append(
                    Violation(
                        file=file,
                        line=line_no,
                        rule="RUST-STYLE-LOG-002",
                        message="Log messages should be complete sentences with capitalization and punctuation.",
                    )
                )

        if len(parts) > 1 and not has_structured_fields(head_text):
            violations.append(
                Violation(
                    file=file,
                    line=line_no,
                    rule="RUST-STYLE-LOG-002",
                    message="Prefer structured logging fields for dynamic context values.",
                )
            )

    return violations


def check_expect_unwrap(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    if "/tests/" in str(file).replace("\\", "/") or file.name.endswith("_test.rs"):
        return violations

    for idx, line in enumerate(lines, start=1):
        code = strip_string_and_line_comment(line)
        code_with_strings = strip_line_comment_preserve_strings(line)

        if UNWRAP_CALL_RE.search(code):
            violations.append(
                Violation(
                    file=file,
                    line=idx,
                    rule="RUST-STYLE-RUNTIME-001",
                    message="Do not use unwrap() in non-test code.",
                )
            )

        expect_match = EXPECT_CALL_RE.search(code_with_strings)
        if expect_match:
            msg = expect_match.group(1).strip()
            if not (msg.startswith('"') and msg.endswith('"')):
                violations.append(
                    Violation(
                        file=file,
                        line=idx,
                        rule="RUST-STYLE-RUNTIME-002",
                        message="expect() must use a clear, user-actionable string literal message.",
                    )
                )
                continue

            text = msg[1:-1].strip()
            if not text:
                violations.append(
                    Violation(
                        file=file,
                        line=idx,
                        rule="RUST-STYLE-RUNTIME-002",
                        message="expect() message must not be empty.",
                    )
                )
                continue

            if not text[0].isupper() or text[-1] not in {".", "!", "?"}:
                violations.append(
                    Violation(
                        file=file,
                        line=idx,
                        rule="RUST-STYLE-RUNTIME-002",
                        message="expect() message should start with a capital letter and end with punctuation.",
                    )
                )

    return violations


def check_numeric_literals(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for idx, line in enumerate(lines, start=1):
        code = strip_string_and_line_comment(line)

        for match in NUM_SUFFIX_RE.finditer(code):
            if match.start() == 0:
                continue
            if code[match.start() - 1] != "_":
                violations.append(
                    Violation(
                        file=file,
                        line=idx,
                        rule="RUST-STYLE-NUM-001",
                        message="Numeric suffixes must be separated by an underscore (for example 10_f32).",
                    )
                )
                break

        for match in PLAIN_INT_RE.finditer(code):
            number = match.group(0)
            if "_" in number:
                continue
            violations.append(
                Violation(
                    file=file,
                    line=idx,
                    rule="RUST-STYLE-NUM-002",
                    message="Integers with more than three digits must use underscore separators.",
                )
            )
            break

    return violations


def check_function_length(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for start, end in find_function_ranges(lines):
        length = end - start + 1
        if length > 120:
            violations.append(
                Violation(
                    file=file,
                    line=start + 1,
                    rule="RUST-STYLE-READ-002",
                    message=f"Function body has {length} lines; keep functions at or under 120 lines.",
                )
            )

    return violations


def check_test_rules(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    for idx, line in enumerate(lines):
        if not TEST_ATTR_RE.match(line):
            continue
        j = idx + 1
        while j < len(lines) and not lines[j].strip():
            j += 1
        if j >= len(lines):
            continue
        fn_match = re.match(r"^\s*fn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(", lines[j])
        if not fn_match:
            continue
        name = fn_match.group(1)
        if not SNAKE_CASE_RE.match(name) or "_" not in name:
            violations.append(
                Violation(
                    file=file,
                    line=j + 1,
                    rule="RUST-STYLE-TEST-001",
                    message="Test function names should be descriptive snake_case.",
                )
            )

    text = "\n".join(lines)
    if re.search(r"^\s*#\s*\[\s*cfg\s*\(\s*test\s*\)\s*]\s*\n\s*mod\s+_test\b", text, flags=re.MULTILINE):
        if re.search(r"mod\s+_test\s*\{[\s\S]*#\s*\[\s*test\s*]", text, flags=re.MULTILINE):
            violations.append(
                Violation(
                    file=file,
                    line=1,
                    rule="RUST-STYLE-TEST-002",
                    message="`#[cfg(test)] mod _test` is reserved for keep-alive imports and must not contain behavior tests.",
                )
            )

    return violations


def check_vertical_spacing(file: Path, lines: list[str]) -> list[Violation]:
    violations: list[Violation] = []

    visited_blocks: set[tuple[int, int]] = set()

    def check_block(start: int, end: int) -> None:
        if end - start < 1:
            return
        key = (start, end)
        if key in visited_blocks:
            return
        visited_blocks.add(key)

        statements = extract_top_level_statements(lines, start, end)
        if not statements:
            return

        last_start, last_end, _ = statements[-1]
        final_is_return_or_tail = is_return_or_tail_statement(lines[last_start : last_end + 1])
        return_like_indices: set[int] = set()
        for i, (stmt_start, stmt_end, _) in enumerate(statements):
            stmt_lines = lines[stmt_start : stmt_end + 1]
            if is_explicit_return_statement(stmt_lines):
                return_like_indices.add(i)
        if final_is_return_or_tail:
            return_like_indices.add(len(statements) - 1)

        for i in range(len(statements) - 1):
            curr_start, curr_end, curr_type = statements[i]
            next_start, next_end, next_type = statements[i + 1]

            # Return-like statements have their own dedicated spacing rule.
            if (i + 1) in return_like_indices:
                continue

            between = lines[curr_end + 1 : next_start]
            blank_count = sum(1 for line in between if not line.strip())

            if curr_type == next_type:
                if blank_count != 0:
                    violations.append(
                        Violation(
                            file=file,
                            line=next_start + 1,
                            rule="RUST-STYLE-SPACE-003",
                            message="Do not insert blank lines within the same statement type.",
                        )
                    )
            elif blank_count != 1:
                violations.append(
                    Violation(
                        file=file,
                        line=next_start + 1,
                        rule="RUST-STYLE-SPACE-003",
                        message="Insert exactly one blank line between different statement types.",
                    )
                )

        for i in sorted(return_like_indices):
            if i == 0:
                continue
            prev_start, prev_end, _ = statements[i - 1]
            ret_start, ret_end, _ = statements[i]
            between = lines[prev_end + 1 : ret_start]
            blank_count = sum(1 for line in between if not line.strip())
            if blank_count != 1:
                stmt_lines = lines[ret_start : ret_end + 1]
                if is_explicit_return_statement(stmt_lines):
                    message = "Insert exactly one blank line before each return statement."
                else:
                    message = "Insert exactly one blank line before the final tail expression."
                violations.append(
                    Violation(
                        file=file,
                        line=ret_start + 1,
                        rule="RUST-STYLE-SPACE-004",
                        message=message,
                    )
                )

        for stmt_start, stmt_end, _stmt_type in statements:
            for child_start, child_end in extract_top_level_brace_blocks_in_span(lines, stmt_start, stmt_end):
                if child_start == start and child_end == end:
                    continue
                if is_data_like_brace_block(lines, child_start, child_end):
                    continue
                check_block(child_start, child_end)

    for start, end in find_function_ranges(lines):
        check_block(start, end)

    return violations


def collect_violations(file: Path) -> list[Violation]:
    lines = file.read_text(encoding="utf-8").splitlines()
    items = parse_top_level_items(lines)

    violations: list[Violation] = []
    violations.extend(check_mod_rs(file))
    violations.extend(check_serde_option_default(file, lines))
    violations.extend(check_error_rs_no_use(file, lines))
    violations.extend(check_import_rules(file, lines, items))
    violations.extend(check_module_order(file, items))
    violations.extend(check_cfg_test_mod_tests_use_super(file, lines))
    violations.extend(check_impl_adjacency(file, lines, items))
    violations.extend(check_impl_rules(file, lines, items))
    violations.extend(check_inline_trait_bounds(file, lines))
    violations.extend(check_std_macro_calls(file, lines))
    violations.extend(check_logging_quality(file, lines))
    violations.extend(check_expect_unwrap(file, lines))
    violations.extend(check_numeric_literals(file, lines))
    violations.extend(check_function_length(file, lines))
    violations.extend(check_vertical_spacing(file, lines))
    violations.extend(check_test_rules(file, lines))
    return violations


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Rust style checker for rust.md rules.")
    parser.add_argument("--check", action="store_true", help="Run style checks.")
    parser.add_argument(
        "--coverage",
        action="store_true",
        help="Print style rule coverage from rust.md rule IDs.",
    )
    parser.add_argument("files", nargs="*", help="Optional list of Rust files to check.")
    return parser.parse_args()


def validate_rule_coverage() -> None:
    missing = STYLE_RULE_IDS - IMPLEMENTED_STYLE_RULE_IDS
    extra = IMPLEMENTED_STYLE_RULE_IDS - STYLE_RULE_IDS
    if missing or extra:
        if missing:
            print(f"Missing style rule implementations: {sorted(missing)}", file=sys.stderr)
        if extra:
            print(f"Unknown implemented style rules: {sorted(extra)}", file=sys.stderr)
        raise SystemExit(2)


def main() -> int:
    validate_rule_coverage()
    args = parse_args()
    if args.coverage:
        for rule in sorted(STYLE_RULE_IDS):
            print(f"{rule}\timplemented")
        return 0

    if not args.check:
        print("Use --check to run validations.")
        return 2

    if args.files:
        files = [Path(path) for path in args.files if path.endswith(".rs")]
    else:
        files = git_tracked_rust_files()

    violations: list[Violation] = []
    for file in files:
        if not file.exists():
            continue
        violations.extend(collect_violations(file))

    if violations:
        for violation in violations:
            print(violation.format())
        print(f"\nFound {len(violations)} style violation(s).", file=sys.stderr)
        return 1

    print(f"Rust style checks passed for {len(files)} file(s).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
