# Herdr und Plugins (Arch/Omarchy und macOS)

Diese Anleitung beschreibt Herdr als Terminal-Workspace für Coding-Agents und zehn getrennte
Herdr-Plugins. Sie ersetzt nicht Herdrs eigene Konfiguration: `~/.config/herdr/config.toml` bleibt
die maßgebliche Live-Datei. Vor Änderungen daran immer die vorhandenen `[keys]`, `[ui]`- und
`[theme]`-Einträge lesen und nur fehlende eigene Einstellungen ergänzen. Alle hier dokumentierten
Plugins sind als `platforms = ["linux", "macos"]` (oder reiner macOS-Plugin-Code) manifestiert und
auf beiden Betriebssystemen getestet; einzelne Linux-only-Workarounds sind explizit markiert.

> Schnellübersicht der Plugin-Tastenkürzel und CLI-Aktionen: [Cheatsheet.md](docs/Cheatsheet.md).

## Was die Plugins tun

| Plugin | Zweck |
|---|---|
| `herdr-sidebar` | Datei-Explorer, Suche und Git-Ansicht als Herdr-Pane. |
| Herdr Radar (`hhdebb.herdr-radar`) | Macht Herdrs Agents-/Spaces-Ansicht lesbarer: Agent, Status und Workspace-Gruppen. Es ist keine eigene Task-App. |
| Herdr Board (`herdr-board`) | Eigenständiges, lokal gespeichertes Ideen- und Aufgabenboard. Agents können Karten über das `board`-CLI lesen und aktualisieren. Keine Firstmate-Abhängigkeit. |
| Memex (`nicosuave.memex`) | Lokaler Session-Desk für Agenten-Historie mit Suche und Resume; OpenCode-Snapshots werden vor Herdr-Memex-Aktionen synchronisiert. |
| Hunk Diff (`jhochenbaum.hunkdiff`) | Inline-Diff-Reviews für Agentenänderungen mit Kommentar-Rückgabe an den zugehörigen Herdr-Agenten. |
| GitHub Ops (`soniiit.github-ops`) | Eigenes Plugin dieses Repos: Issues/PRs durchsuchen, kommentieren, approven, mergen und Reviews/Issues an isolierte Agent-Worktrees delegieren. Bündelt GHZinga 0.5.1. |
| JiraTUI (`drewbitt.jiratui`) | JiraTUI in einem Herdr-Tab, inkl. Issue-Übergabe an laufende Agenten. |
| Terminal Browser (`zenbu-labs.terminal-browser`) | Echter Chromium-Browser im Terminal, als Split neben dem fokussierten Pane. |
| Terminal Code (`zenbu-labs.tode`) | VS Code im Terminal, ersetzt die Shell im fokussierten Pane ohne zusätzlichen Split. |
| Zoetrope (`furkankly.zoetrope`) | Sitzungsgraph einer Claude-Code- oder Codex-Session als Overlay/Split/Tab. OpenCode wird nicht unterstützt. |
| Herdr Auto Title (`herdr.auto-title`) | Benennt Tabs und Panes automatisch nach Projekt, Git-Branch, Programm und Agent-Aufgabe. Manuell vergebene Namen bleiben erhalten. |

Firstmate und Captain’s Deck gehören **nicht** zu diesem Setup. Captain’s Deck ist ein Firstmate-Board
und für ein persönliches Ideenboard nicht erforderlich.

## Voraussetzungen

- Herdr **0.9.0+** mit Socket-Protokoll 22; dieses Setup wurde mit Herdr 0.9.3 geprüft.
- Node.js 22.12+ für Hunk Diff (Herdr Radar allein benötigt Node.js 18+).
- Rust/Cargo für Builds von Herdr-Plugins. Im Mise-Profil dieses Rechners ist `rust = "stable"`
  vorgesehen; nach dem Verlinken der Dotfiles `mise install` ausführen.
- `sqlite3` für den konsistenten lokalen OpenCode-Snapshot, den Memex für die ältere Session-Historie nutzt.
- OpenCode, wenn OpenCode-Karten im Board ausgeführt werden sollen. Die Herdr-Integration muss
  installiert und aktuell sein: `herdr integration status`.
- Für die dynamische Venice-Auswahl müssen `~/.local/bin` im Herdr-`PATH` sowie `venice-model-sync`,
  `opencode` und `herdr`   dort auflösbar sein. Der Board-Öffner benötigt Python 3.11+;
  die Skripte liegen in diesem Repo unter `herdr/bin/` und `opencode/bin/`.
- `gh` (authentifiziert via `gh auth login`) und Git für GitHub Ops.
- `uv` und Python 3.12+ für JiraTUI; die Plugin-Startaktion lädt beim ersten Start die gesperrten
  Laufzeitabhängigkeiten über `uv run --locked --no-dev`. Unter macOS zusätzlich `brew install
  libmagic` (eine JiraTUI-Abhängigkeit lädt es über `python-magic`; unter Linux meist vorinstalliert).
- `jq` für Zoetrope; `zoe` installiert das Plugin selbst über Homebrew oder Cargo, falls es fehlt.
  Zoetrope braucht außerdem eine aktuelle Herdr-Integration für Claude Code oder Codex
  (`herdr integration status`; OpenCode wird nicht unterstützt).

Vor dem Installieren:

```bash
herdr --version
herdr status
node --version
cargo --version
opencode --version
herdr integration status
```

Wenn Cargo fehlt, zuerst prüfen, ob Mise aktiv ist und `rust = "stable"` im verlinkten
`~/.config/mise/config.toml` steht. Keine Systempakete oder eine zweite Toolchain ohne Zustimmung
installieren.

## Plugins installieren

Herdr-Plugins werden mit Herdrs eigenem Installer installiert, **nicht** mit `omarchy plugin add`:

```bash
herdr plugin install alexarthurs/herdr-sidebar/plugins/herdr-sidebar
herdr plugin install hhdebb/herdr-radar
herdr plugin install nelsonPires5/herdr-board --ref v0.19.0
herdr plugin install nicosuave/memex --ref v0.27.1
herdr plugin install jhochenbaum/herdr-hunk-diff --ref v0.4.0
herdr plugin install drewbitt/herdr-jiratui
herdr plugin install zenbu-labs/terminal-browser/herdr-plugin
herdr plugin install zenbu-labs/terminal-code/herdr-plugin
herdr plugin install furkankly/zoetrope/herdr-plugin
herdr plugin install kryptamine/herdr-auto-title
```

Auto Title benötigt nach der Installation einmalig einen Start, weil Herdr Plugins nur beim
Server-Start lädt: `herdr plugin action invoke herdr.auto-title.restart`. Herdr baut das Plugin
aus Go-Quellen (Go 1.24+). Achtung: Der erste Start benennt alle bestehenden Panes einmal um,
danach nur noch unbenannte.

Bei einer interaktiven Installation das Build-Preview prüfen. Die Board-Version ist absichtlich auf
`v0.19.0` festgelegt. Der GitHub-Installer baut das Rust-Plugin und legt das `board`-CLI unter
`~/.local/bin/board` ab. `~/.local/bin` muss im `PATH` der Herdr-Sitzung liegen.

GitHub Ops (`soniiit.github-ops`) ist kein GitHub-Installer-Plugin, sondern lebt als Quellcode in
diesem Repo unter `herdr/plugins/github-ops/` und wird lokal gebaut und verlinkt:

```bash
cd herdr/plugins/github-ops
bash bin/build-ghzinga.sh
herdr plugin link "$PWD"
herdr config check
```

Der Build-Schritt installiert GHZinga 0.5.1 isoliert in `.runtime/`, ohne ein zweites Herdr-Plugin
oder Git's globalen Pager zu verändern.

## Memex: lokale Session-Historie

Memex ist als Keep enthalten und bleibt lokal: Embeddings und Herdr-Startup-Indexierung bleiben aus.
`auto_index_on_search` kann auf dem Memex-Standard bleiben. Die Repo-Datei
`herdr/bin/memex-opencode-snapshot` wird unter `~/.local/bin/memex-opencode-snapshot`
bereitgestellt. Die Memex-Herdr-Aufrufe müssen den Helper vor TUI, Index, Resume und Web ausführen.
Er aktualisiert `opencode-memex-legacy.db` nur, wenn sich die OpenCode-Datenbank oder WAL geändert
hat; die aktive `opencode.db` bleibt unverändert. Falls das installierte Memex-Plugin aktualisiert
wird, die Helper-Einbindung in `herdr/lib.sh`, `herdr/memex-pane.sh` und `herdr/plugin.sh` prüfen.

Die Plugin-Konfiguration liegt unter
`~/.config/herdr/plugins/config/nicosuave.memex/config.toml`; mindestens `index_on_startup = false`
setzen und andere vorhandene Werte erhalten. Das Snapshot-Verfahren entfernt `session_message`
nicht aus der aktiven Datenbank, sondern nur aus einer Kopie, wenn die Projektion leer und keine
`session_v2`-Tabelle vorhanden ist.

Memex öffnen und den OpenCode-Index prüfen:

```bash
herdr plugin action invoke palette --plugin nicosuave.memex
~/.config/herdr/plugins/github/nicosuave.memex-*/bin/memex \
  --no-update-check --non-interactive sessions --count --source opencode --format json
```

Für manuelle Indexläufe den Dotfiles-Wrapper statt des Memex-Binaries direkt verwenden:

```bash
~/.local/bin/memex-opencode-snapshot \
  ~/.config/herdr/plugins/github/nicosuave.memex-*/bin/memex \
  --no-update-check --non-interactive index --only-source opencode --no-embeddings
```

## Hunk Diff: Reviews in Herdr

Hunk Diff v0.4.0 nutzt das mitgelieferte `hunkdiff` 0.22.0. `review.auto_open` ist standardmäßig
aus; der Review startet manuell im fokussierten Agent-Pane oder dessen Worktree. Dadurch wird auch
der zugehörige Agent für `send-review` erfasst.

Die installierbaren Standard-Tasten sind `prefix+shift+h` (Review), `prefix+shift+s` (Kommentare
senden), `prefix+shift+c` (letzten Commit prüfen) und `prefix+shift+a` (gestagte Änderungen prüfen).
`setup-keys` sichert `config.toml`, installiert nur freie Tasten und lässt belegte unverändert. Auf
diesem Rechner ist `prefix+shift+c` bereits `new_workspace`; daher sind Review, Send und Staged auf
`prefix+shift+h/s/a` eingerichtet. Nach erneutem `setup-keys` Herdr-Konfiguration neu laden.

Zum Testen in einem Herdr-Workspace mit Agent-Änderungen:

```bash
herdr plugin action invoke review --plugin jhochenbaum.hunkdiff
herdr plugin action invoke send-review --plugin jhochenbaum.hunkdiff
```

Die Pager-Aktion ist unabhängig: `install-pager` ändert Git's globales `core.pager`; sie wurde hier
nicht ausgeführt.

Herdrs OpenCode-Integration installieren oder aktualisieren, falls `herdr integration status` sie
nicht als aktuell meldet:

```bash
herdr integration install opencode
```

Danach OpenCode-TUIs und bereits laufende gemeinsame OpenCode-Server neu starten, damit die
Integration geladen wird.

## Radar initialisieren, ohne vorhandene Herdr-Anpassungen zu überschreiben

Radar braucht einen laufenden Status-Prozess und die Icon-Schrift:

```bash
herdr plugin action invoke hhdebb.herdr-radar.state-start
herdr plugin action invoke hhdebb.herdr-radar.install-font
```

Falls Agenten-Icons als Kästchen erscheinen, ein neues Terminalfenster öffnen; bei Bedarf das
Terminal selbst neu starten.

**Achtung bei einer bereits angepassten Statusleiste:** `hhdebb.herdr-radar.configure` kann die
gesamte Einrichtung verweigern, wenn `tab_bar_right` oder `[theme.custom]` bereits manuell in
`config.toml` gesetzt ist. Diese Einstellungen nicht löschen oder ersetzen. Wenn nur die Radar-
Agentenzeilen fehlen, das Plugin-Verzeichnis über `~/.config/herdr/plugins.json` ermitteln und
dessen unterstützten Teilmodus verwenden:

```bash
node <plugin-root>/bin/configure.js --rows-on
herdr server reload-config
```

`--rows-on` ergänzt nur die Agent-/Spaces-Zeilen. Bei Konflikten mit bereits eigenen
`[ui.sidebar.*]`-Tabellen stoppen und diese erst gezielt zusammenführen.

## Board für persönliche Ideen

Herdr Board speichert alle Boards lokal in `~/.local/share/herdr-board/board.db`; Karten werden
nicht automatisch ins Git-Repository geschrieben. Im Board:

- `p` öffnet die Projektauswahl; **Global** ist das persönliche, repoübergreifende Board.
- `n` legt eine Karte an, `N` eine Spalte.
- Ein neues Board enthält nur **Todo**, mit `trigger = "manual"`. Karten dort starten keine Agents.
- Eine `auto`-Spalte startet einen Agenten, sobald eine Karte hineingeschoben wird.

Das Beispiel-Pipeline-Template ist für Plan/Execute/Review gedacht: **Plan** schreibt einen Plan in
`docs/plans/` und kann danach automatisch **Execute** starten. Für Ideen oder Read-only-Aufgaben
dieses Template nicht ungeprüft anwenden. Verwende Todo oder eigens angelegte manuelle Spalten.

## OpenCode-Runner für Board-Karten

Der eingebaute OpenCode-TUI-Harness hat auf diesem Rechner trotz angenommener Herdr-Prompt-Übergabe
mehrfach im Idle-Zustand verharrt. Der getestete Workaround verwendet OpenCodes `opencode run` als
konfigurierten Board-Harness. Er bleibt OpenCode, zeigt aber eine Textausgabe im Herdr-Pane statt
des interaktiven OpenCode-TUIs.

Die drei ausführbaren Skripte liegen in diesem Repo unter:

- `herdr/bin/herdr-board-opencode-open`: synchronisiert vor dem Öffnen des Boards den
  Venice-Modellkatalog und aktualisiert ausschließlich `models` in
  `~/.config/herdr-board/config.toml` für `opencode-run`. Danach ruft es Herdrs Board-Aktion auf.
- `herdr/bin/herdr-board-opencode-run`: übergibt die Board-Anweisungen und Kartenbeschreibung
  über eine private temporäre Datei, die nach dem Lauf entfernt wird. Es synchronisiert vor jedem
  `opencode run` erneut die Venice-Modellkonfiguration und setzt **kein** `--auto`.
- `opencode/bin/venice-model-sync`: synchronisiert die Venice-Modell-IDs und Metadaten in
  `~/.config/opencode/opencode.json`; bei Netzwerkfehler bleibt die bestehende Liste erhalten.

Zum Prüfen des Katalogs ohne das Board zu öffnen:

```bash
herdr-board-opencode-open --sync-only
board harness models opencode-run --json
```

Der Board-Öffner liest die Modell-IDs nach dem Sync aus `provider.venice.models` in
`~/.config/opencode/opencode.json`. Wenn der Modellabruf fehlschlägt, bleibt die vorhandene
OpenCode-Konfiguration die Quelle; bei einem ungültigen oder fehlenden Ziel-Eintrag wird das Board
nicht geöffnet und die Konfigurationsdatei nicht überschrieben. Wird die Board-TUI bereits ausgeführt,
muss sie nach dem Sync neu geöffnet werden, damit sie die geänderte Modellauswahl einliest.

Dann in `home/.config/herdr-board/config.toml` eintragen:

```toml
[harness.opencode-run]
argv = ["herdr-board-opencode-run", "{model}"]
models = ["venice/openai-gpt-53-codex"] # wird beim Öffnen des Boards automatisch ersetzt
permission_modes = ["default"]
resume = false
```

Das nutzt die gewählte OpenCode-Modell-ID, lässt Berechtigungen auf **Default** und bietet kein
Auto-Approve an. `herdr-board-opencode-run` muss nach dem Verlinken über Mise/Stow im Herdr-`PATH`
auflösbar sein. Beim Kartenerstellen **Effort auf Standard/leer** und Permission auf **Default**
lassen, sofern für die Karte nichts anderes ausdrücklich nötig ist.

Wenn ein Auto-Lauf fehlschlägt, nicht mehrfach blind wiederholen. `board card show <id> --json`
zeigt Runs/Kommentare; `~/.local/share/herdr-board/logs/` enthält die Prompt-Übergabediagnostik.
Im getesteten Setup kam der eingebaute OpenCode-TUI-Harness wiederholt nicht aus Idle heraus, während
`opencode run` im Board-Pane funktionierte.

## GitHub Ops, JiraTUI, Terminal Browser, Terminal Code, Zoetrope

Detaillierte Bedienung, Fehlerbilder und Beispielaufrufe für alle fünf stehen im
[Cheatsheet.md](docs/Cheatsheet.md). Kurzfassung:

- **GitHub Ops** braucht eine authentifizierte `gh`-CLI. Öffnen:
  `herdr plugin action invoke open --plugin soniiit.github-ops`, oder Strg-Klick auf eine
  GitHub-Issue/PR-URL in einer Herdr-Pane. Approve/Merge verlangen eine getippte Bestätigung.
- **JiraTUI** benötigt vor dem ersten Öffnen eine lokale Konfiguration. **Nicht** `uvx jiratui
  configure create` direkt aufrufen — ein unversioniertes `uvx jiratui` kann eine ältere, im
  `uv`-Cache aufgelöste Version ohne `configure`-Befehl ziehen. Das Herdr-Plugin pinnt
  `jiratui==1.15.0`; dessen gesperrte Umgebung verwenden:
  `uv run --locked --no-dev --project ~/.config/herdr/plugins/github/drewbitt.jiratui-* jiratui configure create`
  (interaktiv, mit den persönlichen Jira-Zugangsdaten). Ohne gültige Konfiguration beendet sich der
  Prozess sofort und der Herdr-Tab schließt sich direkt wieder. Unter macOS zusätzlich vorher
  `brew install libmagic`, sonst bricht der Start mit `ImportError: failed to find libmagic` ab.
- **Terminal Browser** öffnet einen echten Browser als Split; die Adressleiste ist nur über das
  app-lokale `Ctrl+L` (ohne Herdr-Präfix) erreichbar, nicht über die Herdr-Tastenkombination selbst.
- **Terminal Code** ersetzt die Shell im fokussierten Pane statt einen neuen Split zu öffnen. Die
  globale Herdr-Aktion scheitert ohne TTY; das Binding läuft deshalb über
  `herdr pane run "$(herdr pane current | jq -r .result.pane.pane_id)" tode`, das den Befehl in die
  Shell des aktuell fokussierten Panes schickt. **Nur unter Linux/Wayland** zusätzlich
  `ELECTRON_OZONE_PLATFORM_HINT=x11` voranstellen, falls der Split weiß bleibt
  (`--ozone-platform=wayland is not compatible with Vulkan`); unter macOS entfällt das.
- **Zoetrope** unterstützt nur Claude Code und Codex (kein OpenCode) und braucht eine aktuelle
  Herdr-Integration für den jeweiligen Agenten (`herdr integration status`). Keybinding per
  `herdr plugin action invoke setup-keys --plugin furkankly.zoetrope` einrichten lassen, statt es von
  Hand in `config.toml` zu schreiben — das sichert die Konfiguration vorher und prüft freie Tasten.

## Pane-Rahmen und Agents-Liste (Harness-Guides)

Pane-Boxen, automatische Titel und eine lesbare Agents-Liste sind Harness-spezifisch in zwei
Guides dokumentiert – beide decken Radars Sidebar-Block, die Border-Einstellungen in
`~/.config/herdr/config.toml` und die Verifikation ab:

- **[docs/setup-claude-code.md](docs/setup-claude-code.md)** – Claude Code: Titel kommen nativ
  aus dem Terminal-Titel; volle auto-title-Transkript-Features inkl. optionaler `$topic`-Zeile.
- **[docs/setup-opencode.md](docs/setup-opencode.md)** – OpenCode: braucht zusätzlich das
  Plugin `opencode-herdr` (Session-Titel an Herdr) und ein funktionierendes `small_model` für
  die Titel-Generierung; enthält den Venice-`Bad-Request`-Fall und das Fern-Neustarten von TUIs.

Beide Guides nicht mischen: Integration, Plugins und Config-Schritte unterscheiden sich je
Harness. Dieser Rechner folgt dem OpenCode-Guide. Jeder Guide endet mit einem einfügebereiten
**Agent-Prompt**, mit dem der jeweilige Agent (Claude Code bzw. OpenCode) das Setup selbst
ausführt; zusätzlich verweisen die Repo-Dateien `CLAUDE.md` (Claude Code) und `AGENTS.md`
(OpenCode) harness-spezifisch auf den passenden Guide.

## Tastenkürzel

Das aktuelle Präfix dieses Rechners ist `Ctrl+Space` (Herdr-Standard ist `Ctrl+B`). Die vorhandenen
Shortcuts sind:

| Tastenfolge | Funktion |
|---|---|
| Präfix, dann `A` | Radar: gruppiert/flach wechseln |
| Präfix, dann `,` | Radar-Einstellungen |
| Präfix, dann `E` | Explorer öffnen |
| Präfix, dann `B` | Explorer-Sidebar ein-/ausblenden |
| Präfix, dann `Shift+G` | Git-Ansicht öffnen |
| Präfix, dann `Shift+B` | Herdr Board öffnen |
| Präfix, dann `Shift+O` | GitHub Ops öffnen |
| Präfix, dann `Shift+J` | JiraTUI öffnen |
| Präfix, dann `L` | Terminal Browser: Split öffnen |
| Präfix, dann `Shift+T` | Terminal Code im fokussierten Pane öffnen |
| Präfix, dann `Shift+Z` | Zoetrope: Sitzungsgraph (Overlay) |

Board-Binding in `~/.config/herdr/config.toml` (nur hinzufügen, wenn nicht bereits gebunden):

```toml
[[keys.command]]
key = "prefix+shift+b"
type = "shell"
command = "herdr-board-opencode-open"
description = "Open personal Herdr Board"
```

GitHub-Ops-, JiraTUI- und Terminal-Browser/-Code-Bindings (nur hinzufügen, wenn die jeweilige Taste
noch frei ist):

```toml
[[keys.command]]
key = "prefix+shift+o"
type = "shell"
command = "herdr plugin action invoke open --plugin soniiit.github-ops"
description = "GitHub Ops: browse issues and pull requests"

[[keys.command]]
key = "prefix+shift+j"
type = "plugin_action"
command = "drewbitt.jiratui.open"
description = "JiraTUI: öffnen"

[[keys.command]]
key = "prefix+l"
type = "shell"
command = "herdr plugin action invoke zenbu-labs.terminal-browser.open-split"
description = "Terminal Browser: Split öffnen"

[[keys.command]]
key = "prefix+shift+t"
type = "shell"
command = "herdr pane run \"$(herdr pane current | jq -r .result.pane.pane_id)\" tode"
description = "Terminal Code: im fokussierten Pane öffnen"
```

Zoetrope-Binding (`prefix+shift+z`) nicht von Hand eintragen, sondern per
`herdr plugin action invoke setup-keys --plugin furkankly.zoetrope` installieren lassen; das Plugin
verwaltet seinen eigenen Konfigurationsblock.

Nach manuellen Änderungen `herdr config check` ausführen und Herdr-Konfiguration neu laden. Der
konkrete Prefix steht in `[keys].prefix`; bestehende Tasten nicht überschreiben. `prefix+shift+k`
ist auf diesem Rechner bereits `close_workspace`.
