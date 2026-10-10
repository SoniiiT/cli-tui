# OpenCode

Die globale OpenCode-Konfiguration liegt in diesem Repo unter `opencode/config/`
und wird nach `~/.config/opencode/` verlinkt. Der Venice-Modellkatalog wird dort
durch `opencode/bin/venice-model-sync` aktualisiert; der Sync löst den
Konfigurationspfad vor dem atomaren Schreiben auf, damit er den Symlink
nicht ersetzt. Der zugehörige Regressionstest liegt unter `opencode/tests/`.

## Venice Model Router

`opencode-model-router@2.5.0` ist global in `opencode.json` gepinnt. Das aktive
Preset `venice-private` verwendet:

| Tier | Modell | Verwendung |
|---|---|---|
| Hauptmodell | `venice/kimi-k2-7-code` | Orchestrator und normale Unterhaltung |
| `fast` | `venice/z-ai-glm-5-3-flash` | Lesen und Erkunden |
| `medium` | `venice/deepseek-v4-flash-0731` | Implementierung und Refactoring |
| `heavy` | `venice/deepseek-v4-pro-0813` | Architektur und schwieriges Debugging |

Der Router delegiert nach Aufgabe an `fast`, `medium` und `heavy`; das
Hauptmodell bleibt Kimi. `small_model` wird für leichte interne Aufgaben wie
Titelerzeugung verwendet und ist kein allgemeiner Request-Router.

Die teureren, anonymisierten Modelle sind manuelle Agents:

- `@venice-frontier` — Claude Sonnet 5.5
- `@venice-nuke` — Claude Opus 5.5

Weitere manuelle Agents: `@venice-second-opinion` (MiniMax M2.5) und
`@venice-qwen-coder` (Qwen3 Coder 480B). Der Router ruft keinen separaten
Luna-Klassifizierer auf.

Zum Prüfen und Steuern in OpenCode: `/router`, `/tiers`, `/preset venice-private`
und `/budget budget|normal|quality|deep`.

## API-Key

`opencode.json` referenziert `{env:VENICE_API_KEY}`. Den Schlüssel selbst
außerhalb dieses Repositories als Umgebungsvariable setzen; niemals in
Konfigurationsdateien committen.

## Optional: Claude-Code-Credentials-Bridge

`opencode/docs/opencode-claude-auth.opt-in.jsonc` enthält den gepinnten Plugin-Eintrag
`opencode-claude-auth@2.2.1`. Er ist **nicht** in der aktiven Plugin-Liste.
Zum Aktivieren auf einem Arbeitsrechner müsste der Eintrag bewusst in die
`plugin`-Liste der globalen `opencode.json` übernommen werden.

Das ist eine Claude-Code-OAuth-Bridge zu Anthropic, **kein OpenRouter-Plugin**.
Unter Linux liest sie `~/.claude/.credentials.json`; laut Projekt-README kann
sie Tokens zusätzlich in OpenCodes `auth.json` synchronisieren und rotierte
Credentials zurückschreiben. Es wird kein Anthropic-API-Key verwendet, aber
Claude-Code-Abo-Limits werden beansprucht.

Wichtig: Sowohl die Plugin-README als auch OpenCodes Anthropic-Anleitung weisen
darauf hin, dass Claude-Pro/Max- bzw. Claude-Code-Abo-Tokens nur in offiziellen
Anthropic-Clients verwendet werden sollen. Deshalb bleibt diese Bridge hier
deaktiviert. Für Claude über OpenRouter braucht es stattdessen einen separaten
`OPENROUTER_API_KEY` und OpenRouter-Abrechnung.
