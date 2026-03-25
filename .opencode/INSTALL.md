# Installing Tivana Skill for OpenCode

Enable the Tivana browser perception skill in OpenCode.

## Installation

1. **Clone into OpenCode config:**
   ```bash
   git clone https://github.com/Mostrom-LLC/tivana.git ~/.config/opencode/tivana
   ```

2. **Symlink the skills:**
   ```bash
   mkdir -p ~/.config/opencode/skills
   ln -s ~/.config/opencode/tivana/skill ~/.config/opencode/skills/tivana
   ```

3. **Copy the plugin:**
   ```bash
   mkdir -p ~/.config/opencode/plugins
   cp ~/.config/opencode/tivana/.opencode/plugins/tivana.js ~/.config/opencode/plugins/
   ```

4. **Restart OpenCode** to discover the skill.

## Verify

The Tivana skill should appear in the skill list.

## Updating

```bash
cd ~/.config/opencode/tivana && git pull
```

## Uninstalling

```bash
rm ~/.config/opencode/skills/tivana
rm ~/.config/opencode/plugins/tivana.js
rm -rf ~/.config/opencode/tivana
```
