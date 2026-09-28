---
"mdt_cli": fix
---

# Stop watch mode from re-triggering itself

Every scan rewrites the index cache artifact under `.mdt/cache/`, and the recursive watcher treated those writes as project changes — so each check/update scheduled another one after the debounce window, turning watch mode into a busy loop of CPU and disk churn. Watchers now ignore events inside `<root>/.mdt/`. `mdt list` also counts consumers in one pass instead of scanning the consumer list per provider.
