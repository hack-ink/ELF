#!/usr/bin/env python3
from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
TASK_RE = re.compile(r"^\[tasks\.([^\]]+)\]", re.MULTILINE)
CARGO_MAKE_RE = re.compile(r"\bcargo\s+make\s+([A-Za-z0-9][A-Za-z0-9_:-]*)")
MARKDOWN_LINK_RE = re.compile(r"!?\[[^\]\n]*\]\(([^)\n]+)\)")


def read_text(path: Path) -> str:
	return path.read_text(encoding="utf-8")


def makefile_task_names(path: Path, seen: set[Path] | None = None) -> set[str]:
	seen = seen or set()
	path = path.resolve()
	if path in seen:
		return set()
	seen.add(path)

	data = tomllib.loads(read_text(path))
	tasks = set(data.get("tasks", {}))
	for item in data.get("extend", []):
		if not isinstance(item, dict) or not item.get("path"):
			continue
		tasks.update(makefile_task_names(path.parent / item["path"], seen))
	return tasks


def cargo_make_tasks() -> set[str]:
	tasks = makefile_task_names(ROOT / "Makefile.toml")
	if tasks:
		return tasks
	return set(TASK_RE.findall(read_text(ROOT / "Makefile.toml")))


def iter_reference_files() -> list[Path]:
	roots = [
		ROOT / "README.md",
		ROOT / "AGENTS.md",
		ROOT / "docs",
		ROOT / ".github" / "workflows",
	]
	files: list[Path] = []
	for root in roots:
		if root.is_file():
			files.append(root)
			continue
		if root.is_dir():
			files.extend(
				path
				for path in root.rglob("*")
				if path.suffix in {".md", ".yml", ".yaml"}
			)
	return sorted(files)


def iter_markdown_files() -> list[Path]:
	return [
		path
		for path in iter_reference_files()
		if path.suffix == ".md"
	]


def normalize_link_target(raw_target: str) -> str:
	target = raw_target.strip()
	if target.startswith("<") and ">" in target:
		target = target[1:target.index(">")]
	elif " " in target:
		target = target.split(maxsplit=1)[0]
	return target


def is_external_or_anchor(target: str) -> bool:
	return (
		not target
		or target.startswith("#")
		or target.startswith("/")
		or bool(re.match(r"^[A-Za-z][A-Za-z0-9+.-]*:", target))
	)


def check_cargo_make_references(tasks: set[str]) -> list[str]:
	errors: list[str] = []
	for path in iter_reference_files():
		for line_number, line in enumerate(read_text(path).splitlines(), start=1):
			for match in CARGO_MAKE_RE.finditer(line):
				task = match.group(1)
				if task not in tasks:
					rel_path = path.relative_to(ROOT)
					errors.append(f"{rel_path}:{line_number}: unknown cargo make task `{task}`")
	return errors


def check_markdown_links() -> list[str]:
	errors: list[str] = []
	for path in iter_markdown_files():
		for line_number, line in enumerate(read_text(path).splitlines(), start=1):
			for match in MARKDOWN_LINK_RE.finditer(line):
				target = normalize_link_target(match.group(1))
				if is_external_or_anchor(target):
					continue
				path_part = target.split("#", maxsplit=1)[0]
				if not path_part:
					continue
				candidate = (
					ROOT / path_part.removeprefix("/")
					if path_part.startswith("/")
					else path.parent / path_part
				)
				if not candidate.exists():
					rel_path = path.relative_to(ROOT)
					errors.append(f"{rel_path}:{line_number}: broken local link `{target}`")
	return errors


def main() -> int:
	errors = check_cargo_make_references(cargo_make_tasks())
	errors.extend(check_markdown_links())
	if errors:
		for error in errors:
			print(error, file=sys.stderr)
		return 1
	print("check-docs passed")
	return 0


if __name__ == "__main__":
	raise SystemExit(main())
