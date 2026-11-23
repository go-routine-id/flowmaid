#!/usr/bin/env python3
"""Programmatic overprint check for a plain-text dbx ERD capture
(`tmux capture-pane -p`).

* finds the ERD canvas (rows between the '╭ ERD:' border and the
  '│ hjkl/arrows pan' status row, columns inside the frame);
* parses every table box ('╭──name──╮' ... '╰──╯'): the border must be
  intact, the title must be a real table, and every inner line must be
  exactly what dbx's box painter writes for that table's columns (taken
  from sqlite3 PRAGMA table_info / foreign_key_list / index_list), with the
  full "TYPE name" visible and the key tags not overwriting it;
* outside the boxes, every run of text must be exactly one FK label or one
  cardinality token, delimited by space / braille / box-drawing / the
  canvas edge;
* anything else (stray box-drawing, partial tokens, unknown text) is a defect.

usage: check.py capture.txt [...]   -> prints defects, exit 1 if any
       import: check(text) -> (defects, warnings, info)
"""
import json
import re
import unicodedata
import sqlite3
import sys

DB = "/tmp/claude-0/-home-user-flowmaid/cac93b13-7009-5470-94b5-477c61df0333/scratchpad/vid/work-dbx/shop.db"


def load_schema():
    con = sqlite3.connect("file:%s?mode=ro" % DB, uri=True)
    tables, fk_labels = {}, set()
    for (name,) in con.execute("select name from sqlite_master where type='table' and name not like 'sqlite_%'"):
        cols = con.execute("PRAGMA table_info(%s)" % name).fetchall()  # cid,name,type,notnull,dflt,pk
        fks = set()
        for r in con.execute("PRAGMA foreign_key_list(%s)" % name):
            fks.add(r[3])
            fk_labels.add(r[3])
        uniq = set()
        for idx in con.execute("PRAGMA index_list(%s)" % name):  # seq,name,unique,origin,partial
            if idx[2]:
                info = con.execute("PRAGMA index_info('%s')" % idx[1]).fetchall()
                if len(info) == 1:
                    uniq.add(info[0][2])
        rows = []
        for c in cols:
            keys = []
            if c[5]:
                keys.append("PK")
            if c[1] in fks:
                keys.append("FK")
            if c[5] or c[1] in uniq:
                keys.append("UK")
            rows.append((c[2].split("(")[0].strip(), c[1], ",".join(keys)))
        tables[name] = rows
    return tables, fk_labels


TABLES, FK_LABELS = load_schema()
CARDS = {"1", "0..1", "0..N", "1..N"}
TOKENS = FK_LABELS | CARDS


def is_braille(ch):
    return 0x2800 <= ord(ch) <= 0x28FF


def is_box(ch):
    return 0x2500 <= ord(ch) <= 0x257F


def expected_inner(table, H, Wd):
    """Exactly what draw_tables() writes inside a box of inner size H x Wd,
    plus a per-line flag: is the full 'TYPE name' visible and untouched?"""
    rows = TABLES[table]
    out, full = [], []
    visible = min(len(rows), H)
    for r in range(H):
        line = [" "] * Wd
        ok = True
        if r < visible:
            if r == visible - 1 and len(rows) > H:
                s = "+%d more" % (len(rows) - H + 1)
                for i, ch in enumerate(s[:Wd]):
                    line[i] = ch
                ok = len(s) <= Wd
            else:
                ty, name, keys = rows[r]
                left = "%s %s" % (ty, name)
                for i, ch in enumerate(left[:Wd]):
                    line[i] = ch
                ok = len(left) <= Wd
                if keys:
                    kw = len(keys)
                    if Wd > kw:
                        for i, ch in enumerate(keys):
                            line[Wd - kw + i] = ch
                        ok = ok and len(left) + 1 + kw <= Wd
                    else:
                        ok = False  # keys silently dropped: box too narrow
        out.append("".join(line))
        full.append(ok)
    return out, full


ZW = {"\ufe0f", "\ufe0e", "\u200d", "\u200b"}


def cells(line):
    """A captured line as terminal cells (tmux's widths: East Asian Wide /
    Full = 2 cells, combining marks and variation selectors = 0)."""
    out = []
    for ch in line:
        if unicodedata.combining(ch) or ch in ZW:
            continue
        out.append(ch)
        if unicodedata.east_asian_width(ch) in ("W", "F"):
            out.append("")
    return "".join(c if c else "\x00" for c in out)


def find_canvas(lines):
    """ERD canvas in cell coordinates: rows [r0, r1), cols [c0, c1)."""
    top = next(i for i, l in enumerate(lines) if "╭ ERD:" in l)
    left = lines[top].index("╭ ERD:")
    right = lines[top].index("╮", left)
    stat = next(i for i in range(top + 1, len(lines)) if lines[i][left + 1:left + 6] == " hjkl")
    return top + 1, stat, left + 1, right


def check(text):
    lines = [cells(l) for l in text.split("\n")]
    r0, r1, c0, c1 = find_canvas(lines)
    W = c1 - c0
    grid = []
    for r in range(r0, r1):
        l = lines[r]
        if len(l) < c1 + 1 or l[c0 - 1] != "│" or l[c1] != "│":
            return ["row %d: canvas frame broken" % r], [], {}
        grid.append(l[c0:c1])
    H = len(grid)
    defects, warns = [], []
    inbox = [[False] * W for _ in range(H)]
    boxes = []
    for r in range(H):
        for c in range(W):
            if grid[r][c] != "╭" or inbox[r][c]:
                continue
            j = c + 1
            while j < W and grid[r][j] not in "╮╭":
                j += 1
            if j >= W or grid[r][j] != "╮":
                defects.append("box at (%d,%d): no top-right corner" % (r + r0, c + c0))
                continue
            topmid = grid[r][c + 1:j]
            m = re.fullmatch(r"─*([^─]*?)─*", topmid)
            title = m.group(1) if m else None
            # bottom
            i = r + 1
            while i < H and grid[i][c] == "│" and grid[i][j] == "│":
                i += 1
            if i >= H or grid[i][c] != "╰" or grid[i][j] != "╯" or set(grid[i][c + 1:j]) - {"─"}:
                defects.append("box '%s' at (%d,%d): border broken near row %d: %r" % (
                    title, r + r0, c + c0, i + r0, grid[i][c:j + 1] if i < H else ""))
                # find the real bottom (first '╰' below in this column) so
                # the rest of the box is not reported again as stray text
                k = i
                while k < H and grid[k][c] != "╰":
                    k += 1
                if k >= H:
                    continue
                i = k
            for rr in range(r, i + 1):
                for cc in range(c, j + 1):
                    inbox[rr][cc] = True
            boxes.append((title, r + r0, c + c0, i - r + 1, j - c + 1))
            if title not in TABLES:
                defects.append("box at (%d,%d): title %r is not a table" % (r + r0, c + c0, title))
                continue
            Hi, Wd = i - r - 1, j - c - 1
            exp, full = expected_inner(title, Hi, Wd)
            for k in range(Hi):
                got = grid[r + 1 + k][c + 1:j]
                if got != exp[k] and all(g == e or (e == " " and is_braille(g)) for g, e in zip(got, exp[k])):
                    warns.append("box '%s' row %d: edge braille shows through a blank cell" % (title, r + 1 + k + r0))
                elif got != exp[k]:
                    defects.append("box '%s' row %d: %r != expected %r" % (title, r + 1 + k + r0, got.rstrip(), exp[k].rstrip()))
                elif not full[k]:
                    defects.append("box '%s' row %d: column text clipped/keys over text %r" % (title, r + 1 + k + r0, got.rstrip()))
            if c == 0 or j == W - 1:
                warns.append("box '%s' touches the canvas %s edge" % (title, "left" if c == 0 else "right"))
            if r == 0 or i == H - 1:
                warns.append("box '%s' touches the canvas %s edge" % (title, "top" if r == 0 else "bottom"))
    tokens = []
    for r in range(H):
        c = 0
        while c < W:
            ch = grid[r][c]
            if inbox[r][c] or ch == " " or is_braille(ch):
                c += 1
                continue
            if is_box(ch):
                defects.append("stray box-drawing %r at (%d,%d)" % (ch, r + r0, c + c0))
                c += 1
                continue
            s = c
            while c < W and not inbox[r][c] and grid[r][c] != " " and not is_braille(grid[r][c]) and not is_box(grid[r][c]):
                c += 1
            tok = grid[r][s:c]
            tokens.append((tok, r + r0, s + c0))
            if tok in TOKENS:
                if c == W:
                    warns.append("token %r at (%d,%d) ends at the canvas right edge" % (tok, r + r0, s + c0))
                continue
            if c == W and any(t.startswith(tok) for t in TOKENS):
                warns.append("token %r at (%d,%d) clipped by the canvas right edge" % (tok, r + r0, s + c0))
                continue
            defects.append("bad text %r at (%d,%d)" % (tok, r + r0, s + c0))
    info = {"canvas": [r0, r1, c0, c1], "boxes": boxes, "tokens": tokens}
    return defects, warns, info


def main():
    bad = 0
    for f in sys.argv[1:]:
        try:
            d, w, info = check(open(f, encoding="utf-8").read())
        except StopIteration:
            print("%s: no ERD canvas on screen (not an ERD frame)" % f)
            continue
        print("%s: %d defects, %d warnings, %d boxes, %d tokens" % (f, len(d), len(w), len(info.get("boxes", [])), len(info.get("tokens", []))))
        for x in d:
            print("  DEFECT", x)
        for x in w:
            print("  warn  ", x)
        bad += bool(d)
    sys.exit(1 if bad else 0)


if __name__ == "__main__":
    main()
