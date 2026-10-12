# Herdr-Setup: OpenCode Harness

Pane-Boxen, automatische Titel und eine lesbare Agents-Liste für **OpenCode** in Herdr.
Harness-spezifisch, nicht rechner-spezifisch. Für Claude Code siehe
[setup-claude-code.md](setup-claude-code.md).

Getestet mit Herdr 0.9.3 und OpenCode 1.18.31 (V1).

## Voraussetzungen

- Herdr 0.9.0+ (`herdr --version`)
- OpenCode 1.x mit Plugin-Support (`opencode --version`)
- Node.js 18+ (Radar), Go 1.24+ (Herdr baut auto-title aus Quellen)

## 1. Herdr-Integration für OpenCode (zuerst)

```bash
herdr integration install opencode
herdr integration status   # opencode sollte "current" melden
```

Danach OpenCode-TUIs und bereits laufende gemeinsame OpenCode-Server neu starten. Die
Integration meldet Agent-Status und Session an Herdr – sie bleibt die einzige Instanz, die
den Status (working/idle/blocked) setzt; alle weiteren Schritte sind display-only.

## 2. Plugins installieren

```bash
herdr plugin install hhdebb/herdr-radar
herdr plugin install kryptamine/herdr-auto-title
herdr plugin action invoke herdr.auto-title.restart
herdr plugin action invoke hhdebb.herdr-radar.state-start
herdr plugin action invoke hhdebb.herdr-radar.install-font
```

Hinweise zu Radar (Statusleisten-Konflikt, Teilmodus `--rows-on`, Icon-Font) stehen in
[setup-claude-code.md](setup-claude-code.md#2-plugins-installieren) – sie gelten identisch.

## 3. opencode-herdr: echte Session-Titel an Herdr melden

OpenCode meldet seine Session-Titel nicht von selbst an Herdr – ohne dieses Plugin zeigt die
Agents-Liste überall nur `OpenCode`. `justmytwospence/opencode-herdr` meldet den echten
Session-Titel in Herdrs Title-Metadaten-Slot (den Radar automatisch ausliest), zeigt Modell +
Variante als Agent-Namen und bringt ein `$pr`-Token (PR-Status) sowie ein `herdr_child`-Tool mit.

Je eine Zeile, auf denselben Commit gepinnt, in zwei Dateien eintragen:

```jsonc
// ~/.config/opencode/opencode.json  (Server-Hälfte; vorhandene "plugin"-Einträge behalten)
"plugin": ["…", "opencode-herdr@github:justmytwospence/opencode-herdr#f1c0f2b67453"]

// ~/.config/opencode/tui.jsonc  (TUI-Hälfte)
"plugin": ["…", "opencode-herdr@github:justmytwospence/opencode-herdr#f1c0f2b67453"]
```

Das Plugin lädt erst beim OpenCode-Start: danach alle OpenCode-TUIs neu starten. Beim ersten
Start lädt OpenCode das Paket von GitHub. Es ist display-only und ersetzt nicht die offizielle
Integration aus Schritt 1.

## 4. Pflicht: Titel-Generierung prüfen

OpenCode erzeugt Session-Titel über das `small_model` in `~/.config/opencode/opencode.json` –
**nur beim ersten Austausch einer neuen Session, nachträglich nie.** Schlägt die Generierung
fehl, bleiben alle Sessions auf `New session - …`-Platzhaltern und die Agents-Liste zeigt
weiter `OpenCode`.

Prüfen: neue Session starten, einen Prompt schicken, danach

```bash
grep "agent=title" ~/.local/share/opencode/log/opencode.log | tail -3
```

`stream error` beim `agent=title` bedeutet: der Provider lehnt die Title-Agent-Anfrage für
dieses Modell ab. Bekannter Fall (2026-10-12, Venice): `z-ai-glm-5-3-flash` →
`AI_APICallError: Bad Request`, obwohl dasselbe Modell als Hauptmodell funktioniert.
Funktionierende Alternative auf Venice: `small_model = "venice/deepseek-v4-flash"`. Bei anderen
Providern analog ein kleines Modell wählen und erneut prüfen.

Sobald eine Session einen Titel hat, setzt die TUI den Terminal-Titel auf `OC | <titel>`;
Radar zeigt ihn in der Agents-Liste.

## 5. Pane-Boxen in `~/.config/herdr/config.toml`

Identisch zum Claude-Code-Setup, siehe
[setup-claude-code.md](setup-claude-code.md#3-pane-boxen-in-configherdrconfigtoml):
`pane_gaps = true`, `pane_outer_borders = true`, `pane_borders = "always"`,
`show_agent_labels_on_pane_borders = true` unter `[ui]`, danach `herdr config check` und
`herdr server reload-config`.

## 6. OpenCode-TUIs fernsteuerbar neu starten

Nach Plugin- oder Config-Änderungen muss jede TUI neu starten. Aus einem anderen Pane heraus
(nur für TUIs im Leerlauf; Sessions bleiben in der OpenCode-Datenbank erhalten):

```bash
herdr pane process-info --pane <pane-id>     # PID des opencode-Prozesses
kill <pid>                                    # TUI beendet sich, Shell kehrt zurück
herdr pane run <pane-id> "opencode --continue"
```

Die TUI im eigenen Pane kann sich nicht selbst neustarten: dort `/exit` tippen und
`opencode --continue` ausführen.

## 7. Verifizieren

1. In einem Pane `/new`, dann einen Prompt schicken.
2. Agents-Liste: die Zeile zeigt `OC | <generierter Titel>` mit Status-Farbe statt `OpenCode`.
3. `herdr agent list` zeigt in den Tokens `title_working`/`title_done` mit dem Titel.

## Nicht übernehmen aus dem Claude-Code-Setup

- `herdr integration install claude` (hier: `opencode`)
- die `$topic`-Sidebar-Zeile von auto-title (liest Claude-Code-Transkripte; unter OpenCode
  ohne Funktion)

## Agent-Prompt (zum Einfügen in OpenCode)

```text
Richte Herdr auf diesem Rechner für OpenCode ein. Arbeite den Guide
herdr/docs/setup-opencode.md im Repo cli-tui Schritt für Schritt ab
(Integration zuerst, Plugins, opencode-herdr in opencode.json + tui.jsonc,
small_model-Check, Pane-Boxen, Verifikation).

Regeln:
- Vor jeder Änderung an ~/.config/herdr/config.toml und
  ~/.config/opencode/{opencode.json,tui.jsonc} ein Backup anlegen (cp mit
  Datumssuffix); vorhandene plugin-Arrays und [keys]/[ui]/[theme]-Einträge
  lesen und erhalten, nichts Bestehendes überschreiben.
- Radars verwaltete Blöcke (Marker ">>> herdr-radar") nicht von Hand editieren.
- Niemals "herdr server stop" ausführen. OpenCode-TUIs nur im Leerlauf und nur
  nach Schritt 6 des Guides neustarten (process-info -> kill -> opencode
  --continue); niemals die TUI, in der du selbst läufst.
- Den small_model-Check aus Schritt 4 nicht überspringen: neue Session, ein
  Prompt, dann "agent=title" im OpenCode-Log prüfen; bei "stream error" ein
  funktionierendes kleines Modell als small_model setzen (Venice:
  venice/deepseek-v4-flash) und erneut prüfen.
- Keine Schritte aus dem Claude-Code-Guide übernehmen (keine claude-Integration,
  keine $topic-Zeile).
- Am Ende die Verifikation aus dem Guide ausführen und kurz berichten, was
  installiert/konfiguriert wurde und was offen blieb (z. B. eigene TUI erst nach
  manuellem Neustart).
```
