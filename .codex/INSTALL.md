# Installing Tivana Skill for Codex

Enable the Tivana browser perception skill in Codex via native skill discovery.

## Installation

1. **Symlink the skill directory:**
   ```bash
   mkdir -p ~/.agents/skills
   ln -s "$(pwd)/skills/tivana" ~/.agents/skills/tivana
   ```

   **Windows (PowerShell):**
   ```powershell
   New-Item -ItemType Directory -Force -Path "$env:USERPROFILE\.agents\skills"
   cmd /c mklink /J "$env:USERPROFILE\.agents\skills\tivana" "$(Get-Location)\skills/tivana"
   ```

2. **Restart Codex** to discover the skill.

## Verify

```bash
ls -la ~/.agents/skills/tivana
# Should show symlink to skills/tivana/ directory
```

## Updating

If using the repo directly, just `git pull`. Skills update instantly through the symlink.

## Uninstalling

```bash
rm ~/.agents/skills/tivana
```
