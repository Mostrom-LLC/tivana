/**
 * Tivana plugin for OpenCode.ai
 *
 * Auto-registers the Tivana skill directory via config hook.
 */

import path from 'path';
import fs from 'fs';
import { fileURLToPath } from 'url';

const __dirname = path.dirname(fileURLToPath(import.meta.url));

const extractAndStripFrontmatter = (content) => {
  const match = content.match(/^---\n([\s\S]*?)\n---\n([\s\S]*)$/);
  if (!match) return { frontmatter: {}, content };

  const frontmatterStr = match[1];
  const body = match[2];
  const frontmatter = {};

  for (const line of frontmatterStr.split('\n')) {
    const colonIdx = line.indexOf(':');
    if (colonIdx > 0) {
      const key = line.slice(0, colonIdx).trim();
      const value = line.slice(colonIdx + 1).trim().replace(/^["']|["']$/g, '');
      frontmatter[key] = value;
    }
  }

  return { frontmatter, content: body };
};

export const TivanaPlugin = async ({ client, directory }) => {
  const tivanaSkillsDir = path.resolve(__dirname, '../../skills/tivana');

  return {
    config: async (config) => {
      config.skills = config.skills || {};
      config.skills.paths = config.skills.paths || [];
      if (!config.skills.paths.includes(tivanaSkillsDir)) {
        config.skills.paths.push(tivanaSkillsDir);
      }
    },

    'experimental.chat.system.transform': async (_input, output) => {
      const skillPath = path.join(tivanaSkillsDir, 'SKILL.md');
      if (!fs.existsSync(skillPath)) return;

      const fullContent = fs.readFileSync(skillPath, 'utf8');
      const { content } = extractAndStripFrontmatter(fullContent);

      const bootstrap = `<TIVANA_SKILL>
You have the Tivana browser perception skill available.

${content}
</TIVANA_SKILL>`;

      (output.system ||= []).push(bootstrap);
    }
  };
};
