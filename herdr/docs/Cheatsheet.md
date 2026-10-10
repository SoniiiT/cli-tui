# Herdr-Plugin-Cheatsheet

Kurzübersicht zum Öffnen der aktuell installierten Herdr-Plugins auf diesem Rechner.
Das Herdr-Präfix ist **Ctrl+Space**: erst Ctrl+Space drücken und loslassen, dann die angegebene Taste.

## Öffnen und bedienen

| Plugin | Tastenkürzel | Funktion |
|---|---|---|
| Herdr Radar | `Ctrl+Space`, dann `A` | Agentenansicht zwischen „active“ und „recent“ umschalten. Radar ist in Herdrs Agenten-/Spaces-Ansicht integriert, kein eigenes Pane. |
| Herdr Radar | `Ctrl+Space`, dann `,` | Radar-Einstellungen öffnen. |
| herdr-sidebar | `Ctrl+Space`, dann `E` | Explorer-Sidebar öffnen. |
| herdr-sidebar | `Ctrl+Space`, dann `B` | Explorer-Sidebar ein-/ausblenden. |
| herdr-sidebar | `Ctrl+Space`, dann `Shift+G` | Source-Control-/Git-Ansicht öffnen. |
| Herdr Board | `Ctrl+Space`, dann `Shift+B` | Persönliches Board öffnen. Der Shortcut synchronisiert vorher die OpenCode-Modellauswahl. |
| Hunk Diff | `Ctrl+Space`, dann `Shift+H` | Änderungen des fokussierten Agent-Worktrees prüfen. |
| Hunk Diff | `Ctrl+Space`, dann `Shift+S` | Inline-Kommentare an den zugeordneten Agenten senden. |
| Hunk Diff | `Ctrl+Space`, dann `Shift+A` | Gestagte Änderungen prüfen. |
| GitHub Ops | `Ctrl+Space`, dann `Shift+O` | Issues/PRs durchsuchen, kommentieren, approven, mergen, an Agent-Worktree delegieren. |
| JiraTUI | `Ctrl+Space`, dann `Shift+J` | JiraTUI in einem neuen Herdr-Tab öffnen. |
| Terminal Browser | `Ctrl+Space`, dann `L` | Browser in einem Split rechts neben dem fokussierten Pane öffnen. |
| Terminal Code | `Ctrl+Space`, dann `Shift+T` | Tode im bereits fokussierten Herdr-Pane öffnen; Herdr legt keine weitere Aufteilung an. |
| Zoetrope | `Ctrl+Space`, dann `Shift+Z` | Graph einer Claude-Code- oder Codex-Session als Overlay öffnen; OpenCode wird nicht unterstützt. |
| Memex | Kein Shortcut eingerichtet | Session-Palette per Herdr-Aktion; Snapshot-Sync läuft vor dem Öffnen. |

`Ctrl+Space`, dann `Shift+C` bleibt `new_workspace`; Hunk Diff hat für den letzten Commit daher
keinen Shortcut. Die Aktion ist weiter direkt verfügbar:

```bash
herdr plugin action invoke review:commit --plugin jhochenbaum.hunkdiff
```

## Terminal Code öffnen und testen

Zuerst in Herdr die gewünschte Aufteilung anlegen und ein **Shell-Pane am Prompt** fokussieren.
Danach `Ctrl+Space` drücken, loslassen und `Shift+T` drücken. Tode ersetzt die Shell im **bestehenden
Pane**; die Aktion erstellt weder einen zusätzlichen Split noch einen Zoom. Alternativ die Herdr-Aktion
direkt aufrufen:

```bash
herdr plugin action invoke zenbu-labs.tode.open-split
```

Der Launcher sendet `ELECTRON_OZONE_PLATFORM_HINT=x11 tode` an die Shell des fokussierten Herdr-Panes.
Zum Aufteilen wechselst du danach mit Herdr in ein Shell-Pane und erstellst die Splits dort. Manuell
inline starten geht ebenso:

```bash
ELECTRON_OZONE_PLATFORM_HINT=x11 tode
```

Wenn noch eine alte, weiße Wayland-Instanz läuft, zuerst in einem anderen Shell-Pane speichern und
`tode --shutdown` ausführen; das schließt alle Terminal-Code-Fenster. Danach das Ziel-Shell-Pane
fokussieren und die Aktion erneut aufrufen. Beim ersten Start kann Terminal Code Einstellungen,
Tastenkürzel und Erweiterungen importieren wollen; zum Testen kannst du den Import überspringen und
später bei Bedarf mit `tode --import` starten.

## Zoetrope öffnen und testen

**Status: Keep.** Zoetrope ist in `docs/herdr/README.md` und `setup-prompt.md` aufgenommen. Es
unterstützt Claude Code und Codex, aber nicht OpenCode.

Mit Claude Code:

```bash
herdr plugin install furkankly/zoetrope/herdr-plugin
herdr integration status
```

Falls Claude dort nicht als `current` erscheint, installiere die Integration und starte danach eine
neue Claude-Session in Herdr:

```bash
herdr integration install claude
```

Herdr muss die native Session-ID kennen; bereits laufende Claude-Sessions melden sie nicht
rückwirkend. Zoetrope benötigt außerdem `jq`; falls `zoe` fehlt, installiert der Plugin-Build es über
Homebrew oder Cargo.

Fokussiere anschließend das Claude-Pane und öffne das Standard-Overlay:

```bash
herdr plugin action invoke open --plugin furkankly.zoetrope
```

`q` schließt den Graphen und stellt Fokus sowie Layout wieder her. Für andere Platzierungen:

```bash
herdr plugin action invoke open-split --plugin furkankly.zoetrope
herdr plugin action invoke open-tab --plugin furkankly.zoetrope
```

Optional lässt sich `prefix+shift+z` als Overlay-Shortcut einrichten. Auf diesem Rechner entspricht
das **Ctrl+Space**, dann **Shift+Z**:

```bash
herdr plugin action invoke setup-keys --plugin furkankly.zoetrope
```

`setup-keys` sichert die Herdr-Konfiguration, prüft die Belegung und lädt sie nach einer gültigen
Änderung neu. Der Shortcut ist derzeit noch nicht eingerichtet.

## Memex öffnen

Herdr-Startup-Indexierung ist in der Plugin-Konfiguration deaktiviert.
Memex hat davon unabhängig `auto_index_on_search = true`; die erste Palette-Nutzung hat bereits einen
lokalen Scan ausgelöst und `~/.memex` angelegt. Embeddings und Reasoning bleiben standardmäßig aus.
Wenn du auch Auto-Refresh bei Suche abschalten willst, setze `auto_index_on_search = false` in
`~/.memex/config.toml` (hier noch nicht geändert).

Session-Palette als Overlay öffnen:

```bash
herdr plugin action invoke palette --plugin nicosuave.memex
```

Für die große Session-Übersicht:

```bash
herdr plugin action invoke desk --plugin nicosuave.memex
```

Alle lokal unterstützten Agenten-Historien bei Bedarf manuell indexieren:

```bash
herdr plugin action invoke index --plugin nicosuave.memex
```

Das liest lokale Transkripte unterstützter Agents ein und speichert den Suchindex lokal. Der
Index-Action durchsucht standardmäßig alle unterstützten Quellen; Embeddings, Reasoning und
Token-Tracking sind standardmäßig aus. Für eine gezielte Neuindizierung nur von OpenCode:

```bash
~/.local/bin/memex-opencode-snapshot \
  ~/.config/herdr/plugins/github/nicosuave.memex-0a395832d8a8/bin/memex \
  --no-update-check --non-interactive index --only-source opencode --no-embeddings
```

OpenCode wird unterstützt. Auf diesem Rechner hat die aktive DB 42 Session-Einträge und gefüllte
ältere `message`/`part`-Tabellen, aber eine leere `session_message`-Projektion. Memex behandelt die
v2-Projektion als maßgeblich. Deshalb wird eine konsistente Neben-Kopie
`~/.local/share/opencode/opencode-memex-legacy.db` verwendet. Der lokale Snapshot-Helfer aktualisiert
sie automatisch vor dem Öffnen der Herdr-Memex-Panes und vor den Memex-Aktionen für Indexierung,
Resume und Web. Er prüft DB- und WAL-Fingerprints und kopiert nur bei Änderungen; die aktive
`opencode.db` bleibt unverändert. Entfernt wird `session_message` ausschließlich aus der Kopie und
nur wenn die Projektion leer ist und keine `session_v2`-Tabelle existiert. Der aktuelle Memex-Index
enthält 42 Sessions (656 Einträge); Embeddings bleiben aus. Direkte Aufrufe des Memex-Binaries
aktualisieren den Snapshot nicht, daher für manuelle Indexläufe den obigen Wrapper verwenden. Bleibt
ein Memex-Pane offen, während OpenCode neue Inhalte schreibt, das Pane neu öffnen oder `memex:
refresh index` ausführen.

## Hunk Diff öffnen und testen

`Ctrl+Space`, dann `Shift+H` öffnet den Review des fokussierten Herdr-Agenten oder Worktrees. Bei
mehreren Agenten zuerst den gewünschten Agent-Pane fokussieren. In Hunk Inline-Kommentare setzen und
mit `Ctrl+Space`, dann `Shift+S` an den Agenten senden. Der Review öffnet sich nicht automatisch.

Weitere Aktionen:

```bash
herdr plugin action invoke review:staged --plugin jhochenbaum.hunkdiff
herdr plugin action invoke review:branch --plugin jhochenbaum.hunkdiff
herdr plugin action invoke review:commit --plugin jhochenbaum.hunkdiff
herdr plugin action invoke review:stash --plugin jhochenbaum.hunkdiff
```

## Terminal Browser öffnen und testen

Shortcut: `Ctrl+Space`, dann `L`. Alternativ die Herdr-Aktion direkt aufrufen:

```bash
herdr plugin action invoke zenbu-labs.terminal-browser.open-split
```

Das öffnet Terminal Browser rechts neben dem fokussierten Pane. Um eine Adresse einzugeben, klicke
zuerst in diesen Browser-Split und drücke dort **Ctrl+L ohne Herdr-Präfix**. `Ctrl+Space`, dann `L`
öffnet nur den Browser; das Präfix kann den URL-Editor selbst nicht fokussieren. `Ctrl+Q` beendet
Terminal Browser. Die Adressleiste lässt sich umgehen, indem du eine Adresse direkt beim Start
übergibst: `terminal-browser open URL --split right` (URL durch die gewünschte Adresse ersetzen).

Die Installation enthält Terminal Browser **v0.13.4**. Das Setup-Skript wurde bei der Installation
übersprungen. Beachte: Ein normaler erster CLI-Aufruf kann dennoch automatisch die VS-Code-
Bildunterstützung aktivieren und Agent-Skill-Links anlegen. `terminal-browser setup` richtet zusätzlich
Terminal-/Agent-Integration ein; führe es nur aus, wenn du diese Einrichtung möchtest.

## JiraTUI öffnen und testen

Auf diesem Rechner ist die JiraTUI-Standardkonfiguration noch nicht vorhanden. Falls du nicht bereits
eine alternative Konfiguration über `JIRA_TUI_CONFIG_FILE` verwendest, lege sie lokal an:

Ohne gültige JiraTUI-Konfiguration beendet sich der Prozess sofort; der Herdr-Tab kann daher kurz
erscheinen und direkt wieder verschwinden.

**Nicht** `uvx jiratui configure create` direkt aufrufen: Ein unversioniertes `uvx jiratui` zieht die
zuletzt im `uv`-Cache aufgelöste jiratui-Version (hier zunächst 1.6.2), die noch keinen
`configure`-Befehl hatte. Das Herdr-Plugin pinnt `jiratui==1.15.0` in seinem eigenen
`pyproject.toml`/`uv.lock`; nutze dessen gesperrte Umgebung:

```bash
uv run --locked --no-dev --project ~/.config/herdr/plugins/github/drewbitt.jiratui-* jiratui configure create
```

Unter macOS außerdem vorher `libmagic` installieren, sonst bricht der Start mit
`ImportError: failed to find libmagic` ab (eine Abhängigkeit von JiraTUI lädt es über `python-magic`,
das unter Linux meist vorinstalliert ist):

```bash
brew install libmagic
```

Danach in Herdr oder einem Terminal öffnen:

```bash
herdr plugin action invoke drewbitt.jiratui.open
```

JiraTUI öffnet sich in einem neuen Tab. Für die Agenten-Übergabe ein Jira-Issue auswählen,
`Ctrl+Alt+D` drücken, einen verfügbaren Agenten auswählen und mit `Enter` übergeben. `Esc` bricht
die Auswahl ab.

JiraTUI benötigt `uv` und Python **3.12 oder neuer**. `uv` und Python 3.14 sind auf diesem Rechner
vorhanden. Die Plugin-Startaktion lädt beim ersten Start die gesperrten Laufzeitabhängigkeiten.

### JiraTUI-Shortcut

Der Shortcut ist auf diesem Rechner eingerichtet. Auf einem anderen Rechner den folgenden Block zu
`~/.config/herdr/config.toml` hinzufügen, sofern `prefix+shift+j` dort frei ist:

```toml
[[keys.command]]
key = "prefix+shift+j"
type = "plugin_action"
command = "drewbitt.jiratui.open"
description = "JiraTUI: öffnen"
```

Danach die Herdr-Konfiguration neu laden:

```bash
herdr server reload-config
```

## Plugin-Aktionen direkt aufrufen

Plugin-Aktionen lassen sich auch ohne Tastenkürzel starten:

```bash
# Radar: active/recent umschalten oder Einstellungen öffnen
herdr plugin action invoke hhdebb.herdr-radar.view-flip
herdr plugin action invoke hhdebb.herdr-radar.settings

# Sidebar: Explorer, Git, Suche oder Quick Open
herdr plugin action invoke herdr-sidebar.show-explorer
herdr plugin action invoke herdr-sidebar.show-git
herdr plugin action invoke herdr-sidebar.show-search
herdr plugin action invoke herdr-sidebar.quick-open

# JiraTUI in einem neuen Tab öffnen
herdr plugin action invoke drewbitt.jiratui.open

# Terminal Browser in einem Split öffnen
herdr plugin action invoke zenbu-labs.terminal-browser.open-split
```

Für das Board das Tastenkürzel oder `herdr-board-opencode-open` verwenden, damit vor dem Öffnen
die dynamische Modellauswahl aktualisiert wird.

## Diagnose

```bash
herdr plugin list
herdr plugin action list
herdr config check
```

Die Tastenkürzel oben spiegeln die aktuelle Konfiguration dieses Rechners wider. Für die Einrichtung
auf einem anderen Gerät die Shortcut-Blöcke in `~/.config/herdr/config.toml` dort ebenfalls prüfen.
