# Herdr-Setup: Claude Code Harness

Pane-Boxen, automatische Titel und eine lesbare Agents-Liste für **Claude Code** in Herdr.
Harness-spezifisch, nicht rechner-spezifisch. Für OpenCode siehe [setup-opencode.md](setup-opencode.md).

Getestet mit Herdr 0.9.3.

## Voraussetzungen

- Herdr 0.9.0+ (`herdr --version`)
- Claude Code
- Node.js 18+ (Radar), Go 1.24+ (Herdr baut auto-title aus Quellen)
- Icon-Font für Radars Agent-Icons (installiert Radar selbst, s. u.)

## 1. Herdr-Integration für Claude Code (zuerst)

```bash
herdr integration install claude
herdr integration status   # claude sollte "current" melden
```

Danach Claude-Code-Sessions neu starten, damit die Integration geladen wird. Ohne sie meldet
Claude keinen Agent-Status an Herdr, und eine per Slash-Command geöffnete, nie bepromptete
Session bleibt in der Sidebar als generisches `claude` stehen.

## 2. Plugins installieren

```bash
herdr plugin install hhdebb/herdr-radar
herdr plugin install kryptamine/herdr-auto-title
herdr plugin action invoke herdr.auto-title.restart
herdr plugin action invoke hhdebb.herdr-radar.state-start
herdr plugin action invoke hhdebb.herdr-radar.install-font
```

Herdr startet Plugins nur beim Server-Start; die beiden `invoke`-Aufrufe starten sie sofort.
Bei kästchenförmigen Agent-Icons ein neues Terminalfenster öffnen.

**Achtung bei angepasster Statusleiste:** Radars `configure`-Aktion kann die Einrichtung
verweigern, wenn `tab_bar_right` oder `[theme.custom]` in `config.toml` bereits manuell gesetzt
sind. Diese Einträge nicht löschen. Wenn nur die Agentenzeilen fehlen, den unterstützten
Teilmodus nutzen (Plugin-Verzeichnis über `~/.config/herdr/plugins.json` ermitteln):

```bash
node <plugin-root>/bin/configure.js --rows-on
herdr server reload-config
```

## 3. Pane-Boxen in `~/.config/herdr/config.toml`

Unter `[ui]` ergänzen bzw. setzen (vorhandene Einträge lesen, nur Fehlendes ändern):

```toml
[ui]
pane_gaps = true                          # Panes optisch getrennt statt geteilter Trennlinien
pane_outer_borders = true                 # Außenrahmen um den Pane-Bereich
pane_borders = "always"                   # rahmt auch einzelne Panes ein ("auto": nur Splits)
show_agent_labels_on_pane_borders = true  # Agent-/Pane-Name im Rahmen
```

Danach:

```bash
herdr config check
herdr server reload-config
```

Pane manuell umbenennen: Präfix, dann `Shift+P` (oder `herdr pane rename <pane-id> <name>`).

## 4. Wie die Titel bei Claude Code entstehen

Kein Extra-Plugin nötig:

- **Claude Code schreibt die aktuelle Aufgabe selbst in den Terminal-Titel.** Radar liest diesen
  und zeigt ihn als Zeilentitel in der Agents-Liste (mit Status-Farbe und -Icon).
- **herdr-auto-title** benennt Tabs und Panes zusätzlich aus Ordner, Git-Branch und Programm –
  und bei Claude Code vollständig aus den Transkripten: Aufgaben-Titel wie
  `projekt › Implement OAuth scopes`. Manuell vergebene Namen bleiben unangetastet.
- Optional die `$topic`-Sidebar-Zeile (was der Agent im aktiven Tab gerade tut) aktivieren:

```toml
[ui.sidebar.spaces]
rows = [
  ["state_icon", "workspace", "branch", "git_status"],
  [{ token = "$topic", bold = true, dim = false }],
]
```

Achtung: `[ui.sidebar.spaces]` kann zu Radars verwaltetem Block gehören (Marker-Kommentare
`>>> herdr-radar … <<<`). Nur außerhalb der Marker editieren; Konflikte siehe Schritt 2.

## 5. Verifizieren

1. Neue Claude-Code-Session in einem Pane starten und eine Aufgabe geben.
2. Agents-Liste: die Zeile zeigt Logo, Status-Farbe und die Aufgabe statt nur `claude`.
3. Pane-Rahmen zeigt den Pane-Namen; Tab-Titel folgt Ordner/Branch.

## Nicht übernehmen aus dem OpenCode-Setup

- `opencode-herdr` (reines OpenCode-Plugin, läuft unter Claude Code nicht – und ist unnötig,
  weil Claude Code Titel nativ meldet)
- der `small_model`-Fix aus [setup-opencode.md](setup-opencode.md) (OpenCode/Venice-spezifisch)

## Agent-Prompt (zum Einfügen in Claude Code)

```text
Richte Herdr auf diesem Rechner für Claude Code ein. Arbeite den Guide
herdr/docs/setup-claude-code.md im Repo cli-tui Schritt für Schritt ab
(Integration zuerst, dann Plugins, dann Pane-Boxen, dann Verifikation).

Regeln:
- Vor jeder Änderung an ~/.config/herdr/config.toml ein Backup anlegen (cp mit
  Datumssuffix) und vorhandene [keys]/[ui]/[theme]-Einträge lesen; nur fehlende
  eigene Einstellungen ergänzen, nichts Bestehendes überschreiben.
- Radars verwaltete Blöcke (Marker ">>> herdr-radar") nicht von Hand editieren;
  bei Konflikten mit tab_bar_right/theme.custom den --rows-on-Teilmodus nutzen.
- Niemals "herdr server stop" ausführen und keine Herdr-Panes killen, in denen
  ein Agent arbeitet. Config-Änderungen per "herdr config check" +
  "herdr server reload-config" anwenden.
- Keine Schritte aus dem OpenCode-Guide übernehmen (kein opencode-herdr, kein
  small_model-Fix).
- Am Ende die Verifikation aus dem Guide ausführen und kurz berichten, was
  installiert/konfiguriert wurde und was offen blieb.
```
