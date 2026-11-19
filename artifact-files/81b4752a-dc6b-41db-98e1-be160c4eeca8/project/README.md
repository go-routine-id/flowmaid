The personal brand of Arian Saputra, a backend engineer working in Go, as used on ariansaputra.com. The whole site reads like a terminal session: shell prompts open every section, cards are editor windows, experience is a `git log`, and a status bar runs along the bottom. Build anything new for this brand the same way.

Its cards, buttons and page structure follow herdr.dev/tui: square corners, 1px rules everywhere, a ruled page column on a background grid, uppercase mono controls and heavy Archivo headlines. Its colors stay this brand's: one blue, cool greys. How controls respond to hover, focus and press follows Google's Material 3: state layers, a press ripple, a pulsing focus ring and plain tooltips, about a tenth of the system. Use no decoration that isn't a piece of developer tooling.

## Content fundamentals

**Voice.** Plain, specific, engineer to engineer. Lead with what was built and where it runs. Use résumé-style sentences that drop the subject: "Built and runs two production Go platforms, including a multi-tenant PaaS on K3s." Name the real technology (NetworkPolicy, ResourceQuota, K3s, RabbitMQ) rather than calling it "modern infrastructure".

**Honesty about limits** is part of the voice: "Kubernetes at a working level … cluster internals are still developing." Don't oversell.

**Person.** Use first person only where it's natural ("Platforms I own and operate end to end"). Most copy has no pronoun at all.

**Casing.**
- Write labels lowercase in code: nav links (`about`, `experience`, `oss`), button labels (`contact`, `download-cv.pdf`, `ask-my-cv`), hashtags (`#backend-engineer`).
- Controls, eyebrows, stat labels, badges and file names in title bars are *displayed* UPPERCASE with wide tracking by CSS. Nav links and hashtags stay lowercase.
- Titles and prose use sentence case. Product names keep their own casing (EasyRun, go-routine.com).

**Terminal framing.** The brand's prompt is `#![arian]`, written like a Rust inner attribute. Section kickers are commands after it: `#![arian] whoami`, `#![arian] cat ./01_about.md`, `#![arian] cat ./02_ventures.md`. The StatusPanel command uses it too. Tight controls use the short form `#!`: `#! ⌘K`, `#! ASK`. Card title bars carry a file name (`easyrun.md`).

The one exception is the CommandBox. Its line is meant to be copied into a real shell, so it keeps the conventional `$`.

**Punctuation.**
- Use an em dash for asides and in role lines ("Backend Engineer — Go").
- Use a middle dot ` · ` to join facts ("remote · full-time · UTC+7").
- Use an arrow `→` for "go there" ("contact →", feature titles).
- Ranges are written "2026 — present".
- Display headlines and feature titles end in a full stop ("EasyRun.", "Open to work.").
- No exclamation marks.

**Emoji.** Only the 🔥 after an active venture's name. Nowhere else.

## Visual foundations

**Color.** The palette is cool grey neutrals plus one blue.
- `accent` (#1d3fa0 light, #7d96e8 dark) does every job that needs emphasis: the `#![arian]` prompt, links, the one emphasised word in a display headline, the solid button fill, the eyebrow, hover fills, hashtags, role lines, list dashes. Never introduce a second hue (herdr's purple is replaced by this blue).
- Text is `ink` on `paper` or `surface`, with secondary copy in `muted`.
- `faint` fails text contrast, so keep it for decoration (arrows at rest, `##` markers, Contents numbers). Icons that carry meaning, like the Accordion `+`, use `muted`.
- Tinted fills: `accent-soft` behind accent text. Hover and press tints come from Material 3 state layers (below). Text on an `accent` fill is `on-accent`, never literal white.
- `inverse` / `on-inverse` are only for Tooltips.

**The dark chrome.** The nav bar, the profile hero, the status bar, code blocks and the ask button sit on `nav` (#101828) in both themes. Inside them use the `nav-*` tokens: `nav-link` for resting text, `nav-hi` for hover and current, `nav-accent` for emphasis and solid fills, `nav-line` for borders, `hero-hi` for the hero name. Wrap such regions in `.as-on-nav` so Buttons, Chips, the Segmented toggle and CommandBox switch to their on-dark styling.

**Themes.** `light` comes first and `dark` follows. Every token has both values. The NavBar's LIGHT | DARK toggle shows the current one pressed.

**Type.** There are three families from Google Fonts:
- **JetBrains Mono** (400/500/600/700) for UI and ordinary headings: names, section titles, buttons, nav, chips, prompts, labels.
- **Inter** (400/500/600) for reading copy: `body`, `prose`, `lede`, `lead`, `card-body`, `list`, and the `tooltip` label.
- **Archivo** (800/900) for display only, after herdr.dev/tui:
  - `display-hero` 132px / .88 / −.055em
  - `display-end` 70px / .93 / −.05em
  - `feature-num` 44px in `line`
  - `stat-num` 40px
  - `feature-title` 21px weight 800

  Never use Archivo for body text or below 21px.

Mono headings have negative tracking (`hero-name` −.03em, `section-title` −.02em). Small uppercase labels have wide positive tracking: `button` .07em, `label-caps` .2em, `eyebrow` .3em. Body line-height is 1.65, `lede` 1.7, prose 1.75.

**Spacing and layout.**
- Landing pages sit in a **Frame**: one column up to `frame-max` (1440px) with `line-strong` rules down both sides, on a 72px `grid` (`grid-size`). Blocks inside pad with the 34px gutter (`space-34`).
- Reading pages centre content in `wrap-max` (1080px) with `space-20` gutters.
- Below 820px of its own width the NavBar folds its links into a menu sheet behind a 30px menu button. Below 520px it also drops the ⌘K button and tightens its gutter to `space-20`.
- Full sections pad `space-96` / `space-104`; compact ones 40 / 46px.
- The page keeps `statusbar-height` (26px) of bottom padding for the fixed status bar.
- There's no 4/8 grid; the spacing tokens are the values the CSS really repeats.

**Cards and buttons.** These are square, ruled and flat, after herdr.dev/tui.
- Every card, panel, terminal window, command box, chip, badge, button, tooltip, and the code blocks and images in Prose, has `radius-square` corners. Cards, panels, windows, the command box, chips, badges and buttons are bordered with a 1px `line-strong` rule (on the dark chrome, `nav-line`). Nothing casts a shadow except the floating ask button (`elev-fab`) and the crumpled hero edge (`fold-shadow`).
- `line-strong` is as faint as herdr's own rule, 1.5:1 on `paper` light and 1.8:1 dark, under the 3:1 a control border should reach. Every control is identified by its label or glyph instead, and its hover, focus and press states are 3:1 or more. A high-contrast theme would darken it.
- Cards don't move on hover. Linked rows and cells take an `accent` state layer, titles turn `accent`, numerals darken from `line` to `line-strong`, and arrows slide. Plain cards (HighlightCard, Terminal) aren't interactive and get no state layer.
- The **solid** button fills with `accent`. The **ghost** button is a bordered label that fills with `accent` on hover. The **icon** button is a 30px square. All are uppercase mono, 11.5px, .07em.
- The **Segmented** toggle is a ruled group whose pressed option inverts to `ink` on `paper`.
- The **CommandBox** is a ruled line with `$` in `accent` (the only `$` in the system) and `COPY` behind its own rule.
- The **StatusPanel** is the small ruled panel of rows with live / idle dots that sits beside features.

**Lines.** Structure is drawn with 1px rules, not boxes of colour.
- `line-strong` is the structural rule: the Frame's sides, Section, StatStrip and FeatureList boundaries, NavBar and Footer, and every control's border.
- `line` is the quieter rule inside a structure: between FeatureList rows and StatStrip cells, under the Contents label, between Accordion rows, over RuledColumns, and in tables.
- Label rows sit over a block in uppercase (`label-caps`), left and right ("VENTURES" … "CLICK A ROW").
- Prose `h2`s carry a `faint` `##` marker. Links in running text carry a standing `underline` rule.
- Rules are always 1px. Where two ruled blocks meet (a Section above the Footer, a FeatureList above a StatStrip), they share one rule instead of drawing two.
- Only interactive things get state layers and ripples. Plain cards and columns stay still, apart from two quiet cues kept from the site: a Terminal's title bar tints and its first dot lights, and a HighlightCard's numeral and `+` mark darken.

**Paper crease.** The one tactile moment on a page: the hero's sheet ends in a crumpled, uneven fold, and the next block lies under it (**CrumpleEdge**, or Hero `edge="crumple"`).
- The crush is a seeded low-poly sheet. Triangles lit from the top-left give the creases. Its top rows stay flat and fade in, so it joins the hero without a seam.
- The edge is uneven and drops a soft `fold-shadow` (3.5px blur, 4px down) onto the block below. On `paper` the folds use `fold-hi` / `fold-lo`; on the dark `nav` hero they use `nav-fold-hi` / `nav-fold-lo`.
- Use it once per page, only directly under the hero. Never between sections, around cards, or on the cover. The rest of the page stays flat and ruled.
- The same `seed` always draws the same crush. Pick one per page and keep it.
- The block under it drops its top rule: the fold is the boundary.

**Display headlines.** At most one Archivo `display-hero` at the top of a page and one `display-end` at the close. Each has exactly one word in `accent` (`<em>`), an Eyebrow above, and a `lede` below. Everything between uses the mono section titles.

**Interaction states (Material 3).** Every interactive control behaves the way Material 3 specifies (google/material-web tokens). The looks stay square and ruled; only the behaviour comes from Google.
- **State layer.** A layer of the control's content colour sits between its fill and its label. It shows at `state-hover` (8%) on hover and `state-focus` (10%) on keyboard focus. Linked FeatureList rows and StatStrip cells use `accent` for the layer. Controls with a layer: Button, Segmented options, COPY, the nav's ⌘K and menu buttons, AskButton, linked rows and cells.
- **Ripple.** On pointer press a soft radial wave starts at the pointer and spreads across the control at `state-pressed` (10%):
  - it begins at 20% of the larger side
  - it grows to the far corner in 450ms with M3 standard easing, cubic-bezier(.2,0,0,1)
  - it stays at least 225ms, then fades in 375ms
  - there's no ripple for keyboard presses or under `prefers-reduced-motion`
- **Focus ring.** It is `focus-ring-width` (3px) of `accent` at `focus-ring-offset` (2px), or `nav-accent` on the dark chrome. On arrival it swells to 8px and settles back over 600ms. Rows, cells and Segmented options draw it inset.
- **Disabled.** Content drops to `state-disabled` (38%), with no state layer, no ripple and no pointer events.
- **Tooltip.** A plain tooltip on `inverse` with `on-inverse` text in the `tooltip` style (12/16px). It appears after 300ms on hover, or at once on keyboard focus (not on a mouse click), fading over 150ms. It is square, where Material would round it to 4px.
- **Touch.** Hover effects (state layers, ghost fills, arrows, card hovers, tooltips) apply only on devices that can hover, so nothing stays lit after a tap. Press ripples and focus rings work everywhere.

Hovered ghost buttons still fill with `accent` (herdr.dev/tui); the state layer sits on top of whatever fill the control has.

**Motion.** Movement is quick and small:
- 120–140ms colour and fill changes on hover (`--t-in` 140ms)
- `--t-out` 300ms for the timeline reveal
- easing `--e-quiet` cubic-bezier(.22,.61,.36,1), with `--e-expo` cubic-bezier(.16,1,.3,1) for larger reveals

These live in `components/bundle.css`, because the format has no motion tokens, next to the Material 3 values the states use: `--m3-standard` cubic-bezier(.2,0,0,1), `--m3-short3` 150ms and `--m3-long4` 600ms. Arrows slide 3–4px. Nothing lifts or scales. Respect `prefers-reduced-motion` by removing transitions.

**States.**
- Hover: fills and rules turn `accent`; nav links go to `nav-hi`.
- Current: a nav link gets a `·` prefix. A Segmented option is pressed (inverted).
- Selection: `sel-bg` / `sel-fg`.
- Hover, focus, press, disabled: Material 3 state layers, ripple and focus ring (see Interaction states).

**Decoration** is drawn from engineering drawings and circuit boards, always at low strength:
- the background `grid` around the Frame
- the crumpled paper fold under the hero (see Paper crease)
- PCB traces (`pcb-trace`, `pcb-flow`, `pcb-via`) in page corners
- a dimension-line scroll ruler under the nav (`dim-hair`, `dim-tick`, `dim-lbl`)
- `+` registration marks (`reg-mark`)
- a vertical status rail of `rail-txt` ("OPEN TO WORK · REMOTE · FULL-TIME · UTC+7 · YOGYAKARTA")

**Imagery.** There are no photos or illustrations. Visual interest comes from live-looking system diagrams and terminal windows (services as small mono-labelled boxes joined by edges, with packets in `nav-accent`).

## Iconography

There is no icon set. Meaning comes from typed glyphs in the mono face:
- `#![arian]` prompt (`#!` in tight controls); `$` only inside a CommandBox
- `→` go
- `·` separator / current
- `—` list bullet
- `+` mark
- `##` heading
- `⌘K` command palette
- three 1px bars: the NavBar's menu button
- `●` live and `○` idle, drawn as CSS dots in a StatusPanel

The one drawn icon is a small robot head on the ask button. It wasn't captured, so this system leaves it out. Don't substitute a stock icon; use the text label alone.

## Logo

`assets/Logos/as-mark.svg` is the `~/AS` mark: a home-directory path in JetBrains Mono 700, `nav-accent` on a `nav` tile with `radius-mark` corners. In the nav it is set as live text (`~/AS`, 15px, weight 700, .02em, `nav-accent`) rather than the image. The Footer uses the image at 18px.

## Components

`components/bundle.js` exposes `window.AS` and needs React 18. Classes are prefixed `as-`. Use them like this:
- **Frame**: wraps a landing page. The Frame preview shows a whole page.
- **NavBar**, **StatusBar** and **AskButton**: the page frame.
- **Hero**: opens a page. The profile variant is dark; the statement variant follows herdr.dev/tui. `edge="crumple"` ends either one in a **CrumpleEdge**.
- **StatStrip**: figures under the hero.
- **FeatureList** with **StatusPanel**: ventures, capabilities.
- **Section** and **SectionHeader**, with **Eyebrow** and **Statement** for display headings.
- **Terminal**: project and open-source entries.
- **HighlightCard** and **StatTile**: numbered facts and single figures.
- **CommitLog**: work history.
- **CommandBox** and **Segmented**: runnable commands and toggles.
- **Tooltip**: a Material 3 plain tooltip for icon buttons and terse controls.
- **Button**, **Chip**, **HashTag**, **TextLink** and **Badge**: the small parts.
- **Prose**, **Contents**, **Accordion** and **RuledColumns**: reading pages. Prose comes from the site, with herdr.dev's `##` markers; the other three come from herdr.dev's main page and use its quieter `line` rules.
- **Footer**: the closing strip.
