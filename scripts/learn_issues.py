#!/usr/bin/env python3
"""Mirror a learning plan (learn/) into GitHub issues, labels and milestones.

Source of truth stays in the repo: learn/00-PLAN.md (section index),
learn/PROGRESS.md (state) and learn/issues.json (phases, areas, workflow).
GitHub issues are a *view* of that state, so this script is safe to re-run:

  * creates missing labels and milestones (one milestone per phase)
  * creates one issue per section, with a workflow checklist in the body
  * on every run, updates each section issue's managed labels
    (status:*, tier:*, area:*, es:*), milestone and open/closed state
  * never rewrites an issue body after creation, so ticked boxes survive

Requires the GitHub CLI (`gh`), authenticated with write access to the repo.

Usage:
    python3 scripts/learn_issues.py sync            # apply changes
    python3 scripts/learn_issues.py sync --dry-run  # read GitHub, print, change nothing
    python3 scripts/learn_issues.py plan            # offline: print what would be created

Exit codes: 0 = ok, 1 = GitHub/API error, 2 = usage/IO error.
"""

from __future__ import annotations

import argparse
import json
import re
import shutil
import subprocess
import sys
from dataclasses import dataclass, field
from pathlib import Path

LEARN = "learn"
CONFIG = "issues.json"
MARKER = "<!-- learn-section: {n} -->"
MARKER_RE = re.compile(r"<!-- learn-section: (\d+) -->")
TASK_MARKER = "<!-- learn-task: {slug} -->"
TASK_MARKER_RE = re.compile(r"<!-- learn-task: ([a-z0-9-]+) -->")
MANAGED_PREFIXES = ("status:", "tier:", "area:", "es:")

# name -> (colour, description)
LABELS = {
    "type:section": ("1d76db", "One learning-plan section"),
    "type:task": ("5319e7", "Project task outside the section flow"),
    "status:not-written": ("ededed", "Section file not written yet"),
    "status:ready": ("c2e0c6", "Written, not started"),
    "status:in-progress": ("fbca04", "Current section: reading / gate"),
    "status:gate-passed": ("0e8a16", "Gate passed: checkpoint + explain-back"),
    "status:done": ("006b75", "Section complete"),
    "es:pending": ("d4c5f9", "Spanish mirror (learn_es/) not synced"),
    "bonus": ("f9d0c4", "Bonus part: only graded if mandatory is perfect"),
}
TIER_COLOURS = {"T0": "f1f8ff", "T1": "c5def5", "T2": "6fa8dc", "T3": "0b3d91", "TBD": "ededed"}
AREA_COLOUR = "bfd4f2"

DEFAULT_WORKFLOW = [
    "Section file written (`learn/`)",
    "Read the section and answered its questions",
    "Gate passed (misses re-taught, fresh question re-asked)",
    "Checkpoint built",
    "Checkpoint reviewed",
]
DEFAULT_BOOKKEEPING = [
    "`learn/PROGRESS.md` updated (status, gate, review-queue questions)",
    "Section terms added to `GLOSSARY.md` (both languages)",
    "Real decisions logged in `DECISIONS.md`",
    "`python3 scripts/check_learn.py` run",
]


@dataclass
class Section:
    number: int
    file: str
    title: str
    tier: str
    checkpoint: str
    prereqs: list[int] = field(default_factory=list)
    status: str = "status:not-written"
    es_pending: bool = False
    phase: str | None = None
    areas: list[str] = field(default_factory=list)

    @property
    def issue_title(self) -> str:
        return f"§ {self.number} — {self.title}"


# --------------------------------------------------------------------------- repo parsing


def read(path: Path) -> str:
    try:
        return path.read_text(encoding="utf-8")
    except OSError as exc:
        sys.exit(f"error: cannot read {path}: {exc}")


def table_rows(text: str, heading: str) -> list[list[str]]:
    """Return the cells of the first markdown table after a `## heading`."""
    match = re.search(rf"^##\s+{re.escape(heading)}\s*$(.*?)(?=^##\s|\Z)", text, re.M | re.S)
    if not match:
        return []
    rows = []
    for line in match.group(1).splitlines():
        line = line.strip()
        if not line.startswith("|") or re.fullmatch(r"\|[\s|:-]+\|", line):
            continue
        rows.append([c.strip() for c in line.strip("|").split("|")])
    return rows[1:]  # drop header


def section_numbers(cell: str) -> list[int]:
    """'3' -> [3]; '3–19' or '3-19' -> [3..19]."""
    m = re.fullmatch(r"(\d+)\s*[–-]\s*(\d+)", cell)
    if m:
        return list(range(int(m.group(1)), int(m.group(2)) + 1))
    return [int(cell)] if cell.isdigit() else []


def frontmatter_prereqs(path: Path) -> list[int] | None:
    if not path.exists():
        return None
    m = re.search(r"^prereqs:\s*\[(.*?)\]\s*$", read(path), re.M)
    if not m:
        return None
    return [int(x) for x in re.findall(r"\d+", m.group(1))]


def load_sections(root: Path, config: dict) -> list[Section]:
    learn = root / LEARN
    plan = read(learn / "00-PLAN.md")
    progress = read(learn / "PROGRESS.md")

    sections: list[Section] = []
    for cells in table_rows(plan, "Sections"):
        if len(cells) < 6 or not cells[0].isdigit():
            continue
        n = int(cells[0])
        file = cells[1].strip("`")
        prereqs = frontmatter_prereqs(learn / file)
        sections.append(
            Section(
                number=n,
                file=file,
                title=cells[2].replace("`", ""),
                tier=cells[3].strip("*") or "TBD",
                checkpoint=cells[5],
                prereqs=prereqs if prereqs is not None else ([n - 1] if n > 1 else []),
            )
        )
    if not sections:
        sys.exit("error: no rows found under '## Sections' in learn/00-PLAN.md")

    # PROGRESS.md: | # | Status | Gate | Date | ES synced | Notes |
    state: dict[int, tuple[str, str, str]] = {}
    for cells in table_rows(progress, "Sections"):
        if len(cells) < 5:
            continue
        for n in section_numbers(cells[0]):
            state[n] = (cells[1].lower(), cells[2].lower(), cells[4].lower())
    cur = re.search(r"\*\*Current section:\*\*\s*(\d+)", progress)
    current = int(cur.group(1)) if cur else None

    phase_of = {n: p["name"] for p in config.get("phases", []) for n in p["sections"]}
    areas_of: dict[int, list[str]] = {}
    for area, nums in config.get("areas", {}).items():
        for n in nums:
            areas_of.setdefault(n, []).append(area)
    es_mirror = config.get("es_mirror", True)

    for s in sections:
        status, gate, es = state.get(s.number, ("not started", "", ""))
        if status.startswith("done"):
            s.status = "status:done"
        elif s.number == current:
            s.status = "status:gate-passed" if "passed" in gate else "status:in-progress"
        elif "not written" in status:
            s.status = "status:not-written"
        else:
            s.status = "status:ready"
        active = s.status in ("status:done", "status:in-progress", "status:gate-passed")
        s.es_pending = es_mirror and active and not es.startswith("yes")
        s.phase = phase_of.get(s.number)
        s.areas = areas_of.get(s.number, [])
    return sections


# --------------------------------------------------------------------------- GitHub


class GitHub:
    def __init__(self, repo: str, dry_run: bool):
        self.repo = repo
        self.dry_run = dry_run

    def api(self, method: str, path: str, data: dict | None = None):
        if self.dry_run and method != "GET":
            print(f"  [dry-run] {method} {path} {json.dumps(data or {}, ensure_ascii=False)[:160]}")
            return {}
        cmd = ["gh", "api", "-X", method, path]
        if data is not None:
            cmd += ["--input", "-"]
        proc = subprocess.run(
            cmd, input=json.dumps(data) if data is not None else None,
            capture_output=True, text=True,
        )
        if proc.returncode != 0:
            print(f"error: gh api {method} {path}: {proc.stderr.strip()}", file=sys.stderr)
            sys.exit(1)
        return json.loads(proc.stdout) if proc.stdout.strip() else {}

    def get_all(self, path: str) -> list:
        items, page = [], 1
        sep = "&" if "?" in path else "?"
        while True:
            batch = self.api("GET", f"{path}{sep}per_page=100&page={page}")
            items += batch
            if len(batch) < 100:
                return items
            page += 1


def ensure_labels(gh: GitHub, wanted: dict[str, tuple[str, str]]) -> None:
    existing = {l["name"] for l in gh.get_all(f"repos/{gh.repo}/labels")}
    for name, (colour, desc) in wanted.items():
        if name not in existing:
            print(f"+ label {name}")
            gh.api("POST", f"repos/{gh.repo}/labels",
                   {"name": name, "color": colour, "description": desc})


def ensure_milestones(gh: GitHub, phases: list[dict]) -> dict[str, int]:
    existing = {m["title"]: m["number"] for m in gh.get_all(f"repos/{gh.repo}/milestones?state=all")}
    for p in phases:
        if p["name"] not in existing:
            print(f"+ milestone {p['name']}")
            made = gh.api("POST", f"repos/{gh.repo}/milestones",
                          {"title": p["name"], "description": p.get("description", "")})
            existing[p["name"]] = made.get("number", 0)
    return existing


# --------------------------------------------------------------------------- issue content


def checklist(items: list[str], ticked: bool = False) -> str:
    box = "[x]" if ticked else "[ ]"
    return "\n".join(f"- {box} {i}" for i in items)


def section_body(s: Section, config: dict, issue_no: dict[int, int]) -> str:
    repo, branch = config["repo"], config.get("branch", "main")
    done = s.status == "status:done"  # already finished when the issue is first created
    deps = ", ".join(f"#{issue_no[p]}" if p in issue_no else f"§ {p}" for p in s.prereqs) or "—"
    bookkeeping = list(config.get("bookkeeping", DEFAULT_BOOKKEEPING))
    if config.get("es_mirror", True):
        bookkeeping.append("Spanish mirror synced (`learn_es/`)")
    parts = [
        MARKER.format(n=s.number),
        f"**Tier:** {s.tier} · **Phase:** {s.phase or '—'} · "
        f"**Area:** {', '.join(s.areas) or '—'}",
        f"**Material:** [`learn/{s.file}`](https://github.com/{repo}/blob/{branch}/learn/{s.file})",
        f"**Depends on:** {deps}",
        "",
        f"### Done when\n{s.checkpoint}",
        "",
        "### Workflow\n" + checklist(config.get("workflow", DEFAULT_WORKFLOW), done),
    ]
    if config.get("verify"):
        parts += ["", "### Verify\n" + checklist([f"`{c}`" for c in config["verify"]], done)]
    parts += ["", "### Bookkeeping\n" + checklist(bookkeeping, done and not s.es_pending)]
    parts += ["", "_Labels, milestone and open/closed state are synced from "
              "`learn/PROGRESS.md` by `scripts/learn_issues.py`. Tick boxes freely; "
              "the body is never overwritten._"]
    return "\n".join(parts)


def managed_labels(s: Section) -> list[str]:
    labels = ["type:section", s.status, f"tier:{s.tier}"]
    labels += [f"area:{a}" for a in s.areas]
    if s.es_pending:
        labels.append("es:pending")
    if s.phase and "bonus" in s.phase.lower():
        labels.append("bonus")
    return labels


def all_labels(sections: list[Section], config: dict) -> dict[str, tuple[str, str]]:
    wanted = dict(LABELS)
    for t in sorted({s.tier for s in sections}):
        wanted[f"tier:{t}"] = (TIER_COLOURS.get(t, "ededed"), f"Depth tier {t}")
    for a in config.get("areas", {}):
        wanted[f"area:{a}"] = (AREA_COLOUR, f"Code lives in / touches `{a}`")
    return wanted


# --------------------------------------------------------------------------- commands


def cmd_plan(sections: list[Section], config: dict) -> None:
    fake = {s.number: 1000 + s.number for s in sections}
    for s in sections:
        print(f"{s.issue_title}  [{', '.join(managed_labels(s))}]  milestone={s.phase}")
    print(f"\n--- sample body (§ {sections[0].number}) ---\n")
    print(section_body(sections[0], config, fake))
    for t in config.get("tasks", []):
        print(f"\nTASK: {t['title']}  [{', '.join(['type:task'] + t.get('labels', []))}]")


def cmd_sync(sections: list[Section], config: dict, dry_run: bool) -> None:
    if not shutil.which("gh"):
        sys.exit("error: GitHub CLI `gh` not found (brew install gh && gh auth login)")
    gh = GitHub(config["repo"], dry_run)
    ensure_labels(gh, all_labels(sections, config))
    milestones = ensure_milestones(gh, config.get("phases", []))

    issues = [i for i in gh.get_all(f"repos/{gh.repo}/issues?state=all")
              if "pull_request" not in i]
    by_section: dict[int, dict] = {}
    by_task: dict[str, dict] = {}
    for i in issues:
        if m := MARKER_RE.search(i.get("body") or ""):
            by_section[int(m.group(1))] = i
        if m := TASK_MARKER_RE.search(i.get("body") or ""):
            by_task[m.group(1)] = i

    # Pass 1: create missing section issues in order, so "Depends on" can link numbers.
    issue_no = {n: i["number"] for n, i in by_section.items()}
    for s in sections:
        if s.number in by_section:
            continue
        print(f"+ issue {s.issue_title}")
        made = gh.api("POST", f"repos/{gh.repo}/issues", {
            "title": s.issue_title,
            "body": section_body(s, config, issue_no),
            "labels": managed_labels(s),
            **({"milestone": milestones[s.phase]} if s.phase in milestones else {}),
        })
        if made:
            by_section[s.number] = made
            issue_no[s.number] = made["number"]

    # Pass 2: sync labels, milestone and state on every section issue.
    for s in sections:
        issue = by_section.get(s.number)
        if not issue or not issue.get("number"):
            continue
        keep = [l["name"] for l in issue.get("labels", [])
                if not l["name"].startswith(MANAGED_PREFIXES)]
        labels = sorted(set(keep) | set(managed_labels(s)))
        state = "closed" if s.status == "status:done" else "open"
        current = sorted(l["name"] for l in issue.get("labels", []))
        ms = milestones.get(s.phase) if s.phase else None
        cur_ms = (issue.get("milestone") or {}).get("number")
        if labels == current and state == issue["state"] and ms == cur_ms:
            continue
        print(f"~ #{issue['number']} {s.issue_title}: {state}, {s.status}")
        gh.api("PATCH", f"repos/{gh.repo}/issues/{issue['number']}",
               {"labels": labels, "state": state, "milestone": ms})
        if state == "closed" and issue["state"] == "open":
            gh.api("POST", f"repos/{gh.repo}/issues/{issue['number']}/comments",
                   {"body": "Closed by `learn_issues.py`: `PROGRESS.md` marks this section done."})

    # One-off project tasks (created once, then left alone).
    for t in config.get("tasks", []):
        if t["slug"] in by_task:
            continue
        print(f"+ task {t['title']}")
        gh.api("POST", f"repos/{gh.repo}/issues", {
            "title": t["title"],
            "body": TASK_MARKER.format(slug=t["slug"]) + "\n" + t.get("body", ""),
            "labels": ["type:task"] + t.get("labels", []),
            **({"milestone": milestones[t["phase"]]} if t.get("phase") in milestones else {}),
        })
    print("done" + (" (dry run, nothing changed)" if dry_run else ""))


def main() -> None:
    ap = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    ap.add_argument("command", choices=["plan", "sync"])
    ap.add_argument("--dry-run", action="store_true", help="read GitHub but change nothing")
    ap.add_argument("--root", default=".", help="repo root (default: .)")
    args = ap.parse_args()

    root = Path(args.root)
    cfg_path = root / LEARN / CONFIG
    if not cfg_path.exists():
        sys.exit(f"error: {cfg_path} missing — see the learning-plan skill for its format")
    config = json.loads(read(cfg_path))
    for t in config.get("tasks", []):
        for label in t.get("labels", []):
            LABELS.setdefault(label, ("ededed", ""))
    sections = load_sections(root, config)

    if args.command == "plan":
        cmd_plan(sections, config)
    else:
        cmd_sync(sections, config, args.dry_run)


if __name__ == "__main__":
    main()
