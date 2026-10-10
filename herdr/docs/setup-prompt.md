# Copyable AI setup prompt: Herdr on Arch/Omarchy or macOS

Diesen Prompt einem Agenten **auf dem Zielrechner** geben. Er richtet Sidebar, Radar, Herdr Board,
Memex, Hunk Diff, GitHub Ops, JiraTUI, Terminal Browser, Terminal Code und Zoetrope ein, ohne
Firstmate/Captain’s Deck zu installieren oder vorhandene Herdr-/OpenCode-Konfigurationen blind zu
ersetzen. Alle zehn Plugins sind auf Arch/Omarchy und macOS getestet.

---

Du richtest Herdr mit den Plugins `herdr-sidebar`, Herdr Radar, Herdr Board, Memex, Hunk Diff,
`soniiit.github-ops`, JiraTUI, Terminal Browser, Terminal Code und Zoetrope auf diesem Rechner ein.
Lies zuerst `herdr/README.md` in diesem Repo (und bei Bedarf `README.md` im Dotfiles-Repo).
Verwende die Abschnitte dort als Quelle für Shortcuts und den OpenCode-Board-Runner.

## Sicherheits- und Änderungsregeln

- Prüfe zuerst `git status --short --branch` im Dotfiles-Repo und im aktuellen Projekt. Bewahre alle
  uncommitteten Änderungen; editiere oder übertrage sie nicht.
- Prüfe Herdr mit `herdr --version`, `herdr status`, `herdr config check` und
  `herdr integration status`. Starte keinen Herdr-Server neu und schließe keine Workspaces, Tabs oder
  Pane.
- Lies vor jeder Plugin-Installation dessen `herdr-plugin.toml` und die Build-Skripte. Nutze den
  interaktiven Installations-Preview; verwende `--yes` nur nach der ausdrücklichen Zustimmung des
  Benutzers.
- Bevor du Workspaces, Tabs, Pane oder Agents über Herdr steuerst, prüfe `test "${HERDR_ENV:-}" = 1`.
  Wenn der Wert fehlt, keine Herdr-Session kontrollieren; bitte den Benutzer, Herdr im gewünschten
  Setup-Kontext zu öffnen. Read-only CLI-Prüfungen und Plugin-Installation dürfen davon getrennt
  fortgesetzt werden.
- Installiere **nicht** Firstmate oder Captain’s Deck. Das Board hier ist Herdr Board `herdr-board`,
  ein unabhängiges persönliches Board.
- Binde keine globale Auto-Approve-Berechtigung ein. Board-Karten sollen Permission `default` nutzen.
- Schreibe keine Herdr-Standardkonfiguration vollständig neu. Erhalte vorhandene `[keys]`, `[ui]`,
  `[theme]`, `tab_bar_right` und alle unbekannten Einträge.

## Voraussetzungen prüfen

1. Herdr muss mindestens 0.9.0 und Socket-Protokoll 22 verwenden. Eine Herdr-Session nur starten,
   wenn der Benutzer dies wünscht; niemals `herdr server stop` verwenden.
2. Node.js muss mindestens 18 sein.
3. Rust/Cargo wird für den Herdr-Board-Build benötigt. Prüfe `cargo --version` und `rustc --version`;
   falls Rust fehlt, prüfe zunächst `home/.config/mise/config.toml`. Wenn dort `rust = "stable"`
    steht, `mise install` verwenden. Keine Systempakete oder eine zweite Toolchain ohne Zustimmung
    installieren.
4. `sqlite3` muss für den lokalen OpenCode-Snapshot verfügbar sein.
5. Für OpenCode mindestens Version 1.18.29 prüfen. `herdr integration status` muss OpenCode als
    aktuell melden. Falls nicht, erkläre zuerst die persönliche Konfigurationsänderung und frage den
    Benutzer, bevor `herdr integration install opencode` ausgeführt wird.
6. `gh auth status` muss eingeloggt sein (für GitHub Ops).
7. `uv --version` und Python 3.12+ müssen verfügbar sein (für JiraTUI); sonst zuerst `uv` installieren
   (z. B. `brew install uv` auf macOS).
8. `jq --version` muss verfügbar sein (für Zoetrope). `zoe` muss nicht vorab installiert werden, das
   Plugin holt es selbst über Homebrew oder Cargo. Zoetrope braucht außerdem eine aktuelle
   Herdr-Integration für Claude Code oder Codex (`herdr integration status`).

## Plugins installieren

Wenn ein Plugin bereits installiert ist, nicht erneut installieren. Andernfalls die neun
GitHub-Installer-Plugins einzeln installieren und den Herdr-Preview prüfen:

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
```

Falls Rust/Cargo nur über Mise verfügbar ist, muss der Herdr-Build-Prozess `cargo` im `PATH` finden.
Nach dem Build muss `board --version` funktionieren. `~/.local/bin` muss im Benutzer- und Herdr-`PATH`
liegen.

Das zehnte Plugin, `soniiit.github-ops`, ist kein GitHub-Installer-Plugin, sondern lebt als Quellcode
in diesem Repo unter `herdr/plugins/github-ops/` und wird lokal gebaut und verlinkt:

```bash
cd herdr/plugins/github-ops
bash bin/build-ghzinga.sh
herdr plugin link "$PWD"
herdr config check
```

## Memex als Keep einrichten

1. Verlinke `herdr/bin/memex-opencode-snapshot` aus diesem Repo nach
   `~/.local/bin/memex-opencode-snapshot`. Wenn der Zielpfad bereits existiert, vergleiche ihn zuerst;
   nicht überschreiben.
2. Setze in `~/.config/herdr/plugins/config/nicosuave.memex/config.toml` `index_on_startup = false`.
   Erhalte andere vorhandene Plugin-Einstellungen; `auto_index_on_search` kann beim Memex-Standard
   bleiben.
3. Integriere den Snapshot-Helper in die installierte Memex-Herdr-Implementierung: rufe ihn vor dem
   Öffnen der TUI-Panes sowie vor Index-, Resume- und Web-Aufrufen auf. Er muss das Memex-Binary als
   Argumente entgegennehmen und danach ausführen. Prüfe `herdr/lib.sh`, `herdr/memex-pane.sh` und
   `herdr/plugin.sh`; nach einem Memex-Plugin-Upgrade diese Einbindung erneut prüfen.
4. Der Helper erstellt `~/.local/share/opencode/opencode-memex-legacy.db` als konsistente Kopie und
   verändert die aktive OpenCode-Datenbank nie. Nur bei leerem `session_message` ohne `session_v2`
   entfernt er diese Tabelle aus der Kopie. Nie die Originaldatenbank dafür ändern.
5. Prüfe, dass die OpenCode-Sessions indiziert sind:

   ```bash
   ~/.local/bin/memex-opencode-snapshot \
     ~/.config/herdr/plugins/github/nicosuave.memex-*/bin/memex \
     --no-update-check --non-interactive sessions --count --source opencode --format json
   ```

   Der Memex-Index und Embeddings bleiben lokal; keine Cloud-Embeddings aktivieren.

## Hunk Diff als Keep einrichten

1. Hunk Diff benötigt Herdr 0.8+ und Node.js 22.12+. Die installierte Plugin-Version ist v0.4.0
   und bringt `hunkdiff` 0.22.0 mit; keine globale Hunk-Installation ist für Herdr-Reviews nötig.
2. Rufe `herdr plugin action invoke setup-keys --plugin jhochenbaum.hunkdiff` auf. Die Aktion legt
   vor einer Änderung ein Backup von `~/.config/herdr/config.toml` an und überspringt belegte Tasten.
   Danach `herdr server reload-config` ausführen. Auf diesem Rechner ist `prefix+shift+c` bereits
   `new_workspace`; nicht überschreiben. Übrig bleiben Review (`prefix+shift+h`), Send
   (`prefix+shift+s`) und Staged (`prefix+shift+a`).
3. Im Agent-Pane oder im zugehörigen Herdr-Worktree `review` starten. Inline-Kommentare mit Hunk
   setzen; `send-review` schickt sie an den zugeordneten Agenten. `review.auto_open` bleibt aus,
   sodass kein Review-Pane ungefragt geöffnet wird.
4. `install-pager` nicht ausführen, sofern der Benutzer Hunk nicht ausdrücklich als globalen Git-Pager
   möchte; diese Aktion ändert `core.pager`.

## Radar sicher initialisieren

- Installiere die Icon-Schrift und starte den Radar-Lifecycle-Prozess, wenn diese Schritte noch nicht
  erledigt sind:

```bash
herdr plugin action invoke hhdebb.herdr-radar.install-font
herdr plugin action invoke hhdebb.herdr-radar.state-start
```

- Prüfe `~/.config/herdr/config.toml`, bevor du `herdr-radar.configure` ausführst. Wenn eine manuell
  gepflegte `tab_bar_right`-Zeile oder `[theme.custom]` existiert, darf der komplette Configure-Schritt
   diese nicht ersetzen. Folge dann `herdr/README.md` und aktiviere nur die Radar-Agentenzeilen mit
  dem vom installierten Plugin mitgelieferten `configure.js --rows-on`.
- Installiere keine manuell geschriebenen `[ui.sidebar.*]`-Blöcke doppelt. Nach einer Änderung
  `herdr config check` ausführen und Herdr-Konfiguration neu laden.

## Herdr Board als persönliches Ideenboard

1. Lege für den OpenCode-Workaround die Skripte `herdr-board-opencode-open` und
   `herdr-board-opencode-run` aus `herdr/bin/` sowie `venice-model-sync` aus `opencode/bin/`
   unter `~/.local/bin/` ab (Details: `herdr/README.md`). Sie müssen
   ausführbar und über den Herdr-`PATH` gefunden werden. Der Öffner synchronisiert vor dem Board-Start
   die Venice-Modelle und aktualisiert die Modellauswahl in der Board-Konfiguration. Der Runner erhält
   `BOARD_PROMPT`/`BOARD_SYSTEM_PROMPT`, übergibt beides über eine kurzlebige Datei mit Modus 0600 an
   `opencode run`, löscht die Datei danach und verwendet **kein** `--auto`.
2. Lege die Board-Harness-Konfiguration aus `herdr/README.md` unter
   `home/.config/herdr-board/config.toml` ab. Verwende `opencode-run` als eigenen Namen; `[harness.opencode]`
   würde vom eingebauten interaktiven OpenCode-Harness überschattet.
3. Prüfe vor dem Verlinken, ob die Live-Dateien bereits existieren. Wenn ja, vergleiche sie einzeln,
   übernimm nur nach Prüfung und beachte `herdr/README.md`; führe `./bootstrap.sh --adopt` nicht
   ungeprüft aus, da es weitere bestehende Dateien im Home betreffen kann.
4. Füge den Board-Shortcut aus `herdr/README.md` nur hinzu, wenn er nicht belegt ist. Er muss
   `herdr-board-opencode-open` aufrufen, damit vor dem Board-Start die dynamische Modellauswahl
   aktualisiert wird. Auf dem beschriebenen Rechner gilt `prefix = "ctrl+space"`, also öffnet
   `Ctrl+Space`, dann `Shift+B` das Board. `prefix+shift+k` ist dort bereits `close_workspace`; nicht
   überschreiben.
5. Board-Pane öffnen. Für das persönliche, repoübergreifende Board im Projektpicker `p` **Global**
   auswählen. Ein neues Board beginnt mit einer einzelnen manuellen `Todo`-Spalte; Karten mit `n`
   anlegen und optionale manuelle Spalten mit `N` hinzufügen.
6. Für einen Read-only-Test die Karte in `Todo` lassen, Harness `opencode-run`, Modell aus der
   OpenCode-Liste, Effort leer/Standard und Permission `default` setzen. Den gespeicherten Auftrag
   nur erneut starten, wenn kein anderer Run offen ist. Den Beispiel-Pipeline-Shortcut `T` nicht
   anwenden: dessen `Plan`-Spalte schreibt einen Plan unter `docs/plans/` und kann automatisch zu
   `Execute` weitergehen.

## GitHub Ops, JiraTUI, Terminal Browser, Terminal Code und Zoetrope als Keep einrichten

1. **GitHub Ops**: kein zusätzliches Setup nötig außer `gh auth status`. Keybinding nur hinzufügen,
   wenn `prefix+shift+o` frei ist (siehe `herdr/README.md`).
2. **JiraTUI**: Keybinding `prefix+shift+j` (`plugin_action`, Command `drewbitt.jiratui.open`) nur
   hinzufügen, wenn die Taste frei ist. Unter macOS vorher `brew install libmagic` prüfen, sonst
   bricht JiraTUI mit `ImportError: failed to find libmagic` ab. Die Jira-Konfiguration selbst
   verlangt persönliche Zugangsdaten des Benutzers — das nicht automatisiert ausführen, sondern den
   Benutzer bitten, es interaktiv selbst zu tun, und zwar **nicht** mit `uvx jiratui configure
   create` (zieht ggf. eine veraltete, ungepinnte Version ohne `configure`-Befehl aus dem `uv`-Cache),
   sondern über die vom Plugin gesperrte Umgebung:
   `uv run --locked --no-dev --project ~/.config/herdr/plugins/github/drewbitt.jiratui-* jiratui configure create`.
3. **Terminal Browser**: Keybinding `prefix+l` (shell, `herdr plugin action invoke
   zenbu-labs.terminal-browser.open-split`) nur hinzufügen, wenn die Taste frei ist. Keine
   zusätzlichen CLI-Aufrufe vor dem ersten `herdr plugin install` ausführen (`terminal-browser ls` o. ä.
   kann automatisch VS-Code-Bildunterstützung aktivieren und Agent-Skill-Links anlegen).
4. **Terminal Code**: Keybinding `prefix+shift+t` nur hinzufügen, wenn die Taste frei ist. Die globale
   Plugin-Aktion scheitert ohne TTY; das Binding muss daher über `herdr pane run` in die Shell des
   fokussierten Panes schicken:

   ```toml
   [[keys.command]]
   key = "prefix+shift+t"
   type = "shell"
   command = "herdr pane run \"$(herdr pane current | jq -r .result.pane.pane_id)\" tode"
   description = "Terminal Code: im fokussierten Pane öffnen"
   ```

   Nur unter Linux/Wayland zusätzlich `ELECTRON_OZONE_PLATFORM_HINT=x11 ` vor `tode` setzen, falls der
   Split weiß bleibt; unter macOS nicht nötig.
5. **Zoetrope**: braucht `jq` und eine aktuelle Herdr-Integration für Claude Code oder Codex
   (OpenCode wird nicht unterstützt). Keybinding **nicht** von Hand eintragen, sondern per
   `herdr plugin action invoke setup-keys --plugin furkankly.zoetrope` installieren lassen — das
   Plugin sichert die Konfiguration vorher und überspringt belegte Tasten automatisch.
6. Nach jeder Keybinding-Änderung `herdr config check` und danach `herdr server reload-config`
   ausführen.

## Abschlussprüfung

```bash
herdr plugin list
herdr integration status
herdr config check
board version --json
board harness list --json
board daemon status --json
```

`herdr plugin list` muss alle zehn Plugins als `enabled` zeigen.

`board harness list` muss `opencode-run` enthalten. Führe höchstens einen klar beschriebenen
Read-only-Testlauf aus. Wenn OpenCode im leeren Welcome-Prompt bleibt, nicht wiederholt retryen:
`board card show <id> --json` und die Board-Logs prüfen; bestehende Projektänderungen unangetastet
lassen.

Am Ende fasse zusammen, welche Plugins/Harness-Integrationen installiert wurden, welches Board und
welcher Shortcut eingerichtet wurden, welche Prüfungen bestanden sind und welche Schritte noch
Benutzerfreigabe benötigen.
