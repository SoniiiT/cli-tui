import json
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "bin/venice-model-sync"


class VeniceModelSyncSymlinkTests(unittest.TestCase):
    def test_atomic_sync_keeps_stow_symlink_and_updates_its_source(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            repo_config = root / "repo" / "opencode.json"
            live_config = root / "home" / ".config" / "opencode.json"
            backup_dir = root / "home" / ".config" / ".backups"
            repo_config.parent.mkdir(parents=True)
            live_config.parent.mkdir(parents=True)
            repo_config.write_text(
                json.dumps(
                    {
                        "model": "venice/kimi-k2-7-code",
                        "small_model": "venice/z-ai-glm-5-3-flash",
                        "plugin": ["opencode-model-router@2.5.0"],
                        "provider": {"venice": {"models": {"retired": {}}}},
                    }
                ),
                encoding="utf-8",
            )
            live_config.symlink_to(repo_config)

            namespace = {"__file__": str(SCRIPT), "__name__": "venice_model_sync_test"}
            exec(compile(SCRIPT.read_bytes(), str(SCRIPT), "exec"), namespace)
            namespace["CONFIG_PATH"] = str(live_config)
            namespace["BACKUP_DIR"] = str(backup_dir)
            namespace["fetch_venice_models"] = lambda: [
                {
                    "id": "active-model",
                    "model_spec": {
                        "name": "Active model",
                        "capabilities": {"supportsReasoning": True},
                        "maxCompletionTokens": 4096,
                        "pricing": {
                            "input": {"usd": 0.1},
                            "output": {"usd": 0.2},
                        },
                    },
                }
            ]

            self.assertEqual(namespace["main"](), 0)

            self.assertTrue(live_config.is_symlink())
            self.assertEqual(live_config.resolve(), repo_config)
            synced = json.loads(repo_config.read_text(encoding="utf-8"))
            self.assertEqual(synced["model"], "venice/kimi-k2-7-code")
            self.assertEqual(synced["small_model"], "venice/z-ai-glm-5-3-flash")
            self.assertEqual(synced["plugin"], ["opencode-model-router@2.5.0"])
            self.assertIn("active-model", synced["provider"]["venice"]["models"])
            self.assertEqual(len(list(backup_dir.glob("opencode.json.*.bak"))), 1)


if __name__ == "__main__":
    unittest.main()
