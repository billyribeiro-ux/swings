#!/usr/bin/env python3
"""Generate project-audit.md with verbatim file contents per audit specification."""

from __future__ import annotations

from pathlib import Path

REPO = Path(__file__).resolve().parents[1]
OUT_MAIN = REPO / "project-audit.md"
# Rotate to part2+ if a single output would grow too large (rare for this repo).
MAX_BYTES_PER_FILE = 9 * 1024 * 1024


def read_text(path: Path) -> str:
    return path.read_text(encoding="utf-8", errors="replace")


def fence(lang: str, path: str, body: str) -> str:
    return f"```{lang}\n// {path}\n{body.rstrip()}\n```\n\n"


def os_walk_tree(root: Path, display_root: str) -> str:
    """Nested tree using directory walk (matches common tree-style output)."""
    lines: list[str] = ["```", display_root.rstrip("/") + "/"]

    def walk(dir_path: Path, prefix: str) -> None:
        try:
            entries = sorted(dir_path.iterdir(), key=lambda p: (p.is_file(), p.name.lower()))
        except OSError:
            return
        for i, path in enumerate(entries):
            is_last = i == len(entries) - 1
            connector = "└── " if is_last else "├── "
            lines.append(f"{prefix}{connector}{path.name}")
            if path.is_dir():
                ext = "    " if is_last else "│   "
                walk(path, prefix + ext)

    walk(root, "")
    lines.append("```")
    return "\n".join(lines) + "\n\n"


class Writer:
    def __init__(self, path: Path) -> None:
        self.path = path
        self.f = path.open("w", encoding="utf-8")
        self.part_idx = 1
        self.bytes_written = 0

    def write(self, s: str) -> None:
        b = s.encode("utf-8")
        if self.bytes_written + len(b) > MAX_BYTES_PER_FILE:
            self.f.close()
            self.part_idx += 1
            self.path = REPO / f"project-audit-part{self.part_idx}.md"
            self.f = self.path.open("w", encoding="utf-8")
            self.bytes_written = 0
            self.f.write(f"# Project Audit (continued — part {self.part_idx})\n\n")
        self.f.write(s)
        self.bytes_written += len(b)

    def close(self) -> None:
        self.f.close()


def main() -> None:
    w = Writer(OUT_MAIN)

    w.write("# Project Audit\n\n")

    w.write("## Section 1: Frontend\n\n")

    w.write("### 1.1 package.json\n\n")
    w.write(fence("json", "package.json", read_text(REPO / "package.json")))

    w.write("### 1.2 svelte.config.js\n\n")
    w.write(fence("javascript", "svelte.config.js", read_text(REPO / "svelte.config.js")))

    w.write("### 1.3 vite.config.ts\n\n")
    w.write(fence("typescript", "vite.config.ts", read_text(REPO / "vite.config.ts")))

    w.write("### 1.4 tsconfig.json\n\n")
    w.write(fence("json", "tsconfig.json", read_text(REPO / "tsconfig.json")))

    w.write("### 1.5 src/app.html\n\n")
    w.write(fence("html", "src/app.html", read_text(REPO / "src" / "app.html")))

    w.write("### 1.6 src/app.d.ts\n\n")
    w.write(fence("typescript", "src/app.d.ts", read_text(REPO / "src" / "app.d.ts")))

    w.write("### 1.7 Directory tree of `src/`\n\n")
    w.write(os_walk_tree(REPO / "src", "src"))

    w.write("### 1.8 Layout files (`+layout.svelte`, `+layout.ts`, `+layout.server.ts`)\n\n")
    layouts = sorted(
        p
        for p in (REPO / "src").rglob("*")
        if p.is_file()
        and p.name.startswith("+layout")
        and p.suffix in (".svelte", ".ts")
    )
    for p in layouts:
        rel = p.relative_to(REPO).as_posix()
        lang = "svelte" if p.suffix == ".svelte" else "typescript"
        w.write(f"#### `{rel}`\n\n")
        w.write(fence(lang, rel, read_text(p)))
    w.write(
        "**Note:** No `+layout.server.ts` files exist in this repository.\n\n"
    )

    w.write("### 1.9 `+page.server.ts` and `+server.ts` files\n\n")
    bff = sorted(
        p
        for p in (REPO / "src").rglob("*")
        if p.is_file() and p.name in ("+page.server.ts", "+server.ts")
    )
    for p in bff:
        rel = p.relative_to(REPO).as_posix()
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("typescript", rel, read_text(p)))

    w.write("### 1.10 API client and related files (`src/lib/api/`)\n\n")
    api_dir = REPO / "src" / "lib" / "api"
    for p in sorted(api_dir.iterdir()):
        if p.is_file():
            rel = p.relative_to(REPO).as_posix()
            lang = "typescript" if p.suffix == ".ts" else "text"
            w.write(f"#### `{rel}`\n\n")
            w.write(fence(lang, rel, read_text(p)))

    w.write("### 1.11 Auth-related frontend files\n\n")
    auth_path = REPO / "src" / "lib" / "stores" / "auth.svelte.ts"
    rel = auth_path.relative_to(REPO).as_posix()
    w.write(f"#### `{rel}`\n\n")
    w.write(fence("typescript", rel, read_text(auth_path)))

    w.write("### 1.12 `src/hooks.server.ts` and `src/hooks.client.ts`\n\n")
    for name in ("src/hooks.server.ts", "src/hooks.client.ts"):
        p = REPO / name
        w.write(f"#### `{name}`\n\n")
        w.write(fence("typescript", name, read_text(p)))

    w.write("### 1.13 Shared / API TypeScript definitions\n\n")
    types_path = REPO / "src" / "lib" / "api" / "types.ts"
    tr = types_path.relative_to(REPO).as_posix()
    w.write(f"#### `{tr}`\n\n")
    w.write(fence("typescript", tr, read_text(types_path)))
    w.write(
        "**Note:** No `src/lib/types/` or `src/lib/models/` directories exist in this repo.\n\n"
    )

    w.write("## Section 2: Backend (Rust)\n\n")

    bsrc = REPO / "backend" / "src"
    main_rs = read_text(bsrc / "main.rs")

    w.write("### 2.1 backend/Cargo.toml\n\n")
    w.write(fence("toml", "backend/Cargo.toml", read_text(REPO / "backend" / "Cargo.toml")))

    w.write("### 2.2 Directory tree of `backend/src/`\n\n")
    w.write(os_walk_tree(bsrc, "backend/src"))

    w.write("### 2.3 backend/src/main.rs\n\n")
    w.write(fence("rust", "backend/src/main.rs", main_rs))

    w.write("### 2.4 Application state (`AppState` in `backend/src/main.rs`)\n\n")
    w.write(
        "The `AppState` struct and server setup live in `backend/src/main.rs`. "
        "Full file contents (verbatim duplicate of §2.3):\n\n"
    )
    w.write(fence("rust", "backend/src/main.rs", main_rs))

    w.write("### 2.5 Route handler files (`backend/src/handlers/`)\n\n")
    hdir = bsrc / "handlers"
    for p in sorted(hdir.glob("*.rs")):
        rel = p.relative_to(REPO).as_posix()
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("rust", rel, read_text(p)))

    w.write("### 2.6 Services, models, persistence, and configuration\n\n")
    for rel in (
        "backend/src/services/mod.rs",
        "backend/src/services/storage.rs",
        "backend/src/models.rs",
        "backend/src/db.rs",
        "backend/src/config.rs",
        "backend/src/error.rs",
    ):
        p = REPO / rel
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("rust", rel, read_text(p)))

    w.write("### 2.7 Middleware and extractors\n\n")
    for rel in (
        "backend/src/middleware.rs",
        "backend/src/middleware/rate_limit.rs",
        "backend/src/extractors.rs",
    ):
        p = REPO / rel
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("rust", rel, read_text(p)))

    w.write("### 2.8 Email service (`backend/src/email.rs`)\n\n")
    w.write(
        "**Note:** Email HTML templates are embedded as string constants in this file "
        "(no separate `templates/` directory).\n\n"
    )
    w.write(fence("rust", "backend/src/email.rs", read_text(bsrc / "email.rs")))

    w.write("### 2.9 Stripe integration and webhooks\n\n")
    for rel in ("backend/src/stripe_api.rs", "backend/src/handlers/webhooks.rs"):
        p = REPO / rel
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("rust", rel, read_text(p)))

    w.write("### 2.10 backend/.env.example\n\n")
    w.write(fence("dotenv", "backend/.env.example", read_text(REPO / "backend" / ".env.example")))

    w.write("### 2.11 backend/Dockerfile\n\n")
    w.write(fence("dockerfile", "backend/Dockerfile", read_text(REPO / "backend" / "Dockerfile")))

    w.write("## Section 3: Database Migrations\n\n")
    mig_dir = REPO / "backend" / "migrations"
    migrations = sorted(mig_dir.glob("*.sql"))
    for i, p in enumerate(migrations, start=1):
        rel = p.relative_to(REPO).as_posix()
        w.write(f"### 3.{i} `{rel}`\n\n")
        w.write(fence("sql", rel, read_text(p)))

    w.write("## Section 4: Configuration\n\n")

    w.write("### 4.1 `backend/render.yaml`\n\n")
    p = REPO / "backend" / "render.yaml"
    if p.exists():
        w.write(fence("yaml", "backend/render.yaml", read_text(p)))
    else:
        w.write("**File does not exist:** `backend/render.yaml`\n\n")

    w.write("### 4.2 Environment template files (project root)\n\n")
    for name in (".env.example", ".env.local.example"):
        p = REPO / name
        if p.exists():
            w.write(f"#### `{name}`\n\n")
            w.write(fence("dotenv", name, read_text(p)))
        else:
            w.write(f"**File does not exist:** `{name}`\n\n")

    w.write("### 4.3 Docker Compose\n\n")
    dc_yml = REPO / "docker-compose.yml"
    dc_yaml = REPO / "docker-compose.yaml"
    if dc_yml.exists():
        w.write(f"#### `docker-compose.yml`\n\n")
        w.write(fence("yaml", "docker-compose.yml", read_text(dc_yml)))
    elif dc_yaml.exists():
        w.write(f"#### `docker-compose.yaml`\n\n")
        w.write(fence("yaml", "docker-compose.yaml", read_text(dc_yaml)))
    else:
        w.write("**File does not exist:** `docker-compose.yml` / `docker-compose.yaml`\n\n")

    w.write("### 4.4 `vercel.json`\n\n")
    vp = REPO / "vercel.json"
    if vp.exists():
        w.write(fence("json", "vercel.json", read_text(vp)))
    else:
        w.write("**File does not exist:** `vercel.json`\n\n")

    w.write("### 4.5 `.gitignore`\n\n")
    w.write(fence("gitignore", ".gitignore", read_text(REPO / ".gitignore")))

    w.write("### 4.6 README files\n\n")
    for rel in ("README.md", "backend/README.md"):
        p = REPO / rel
        w.write(f"#### `{rel}`\n\n")
        w.write(fence("markdown", rel, read_text(p)))

    w.close()

    part_files = sorted(REPO.glob("project-audit-part*.md"))
    if part_files:
        tail = (
            "\n\n---\n\n**Continued in:** "
            + ", ".join(f"`{p.name}`" for p in part_files)
            + "\n"
        )
        OUT_MAIN.write_text(OUT_MAIN.read_text(encoding="utf-8") + tail, encoding="utf-8")

    print("Wrote:", OUT_MAIN)
    for pf in part_files:
        print("Wrote:", pf)


if __name__ == "__main__":
    main()
