import type { BaseLayoutProps } from 'fumadocs-ui/layouts/shared';

export function baseOptions(): BaseLayoutProps {
  return {
    nav: {
      title: 'Tivana',
    },
    githubUrl: 'https://github.com/Mostrom-LLC/tivana',
    links: [
      {
        text: 'npm',
        url: 'https://www.npmjs.com/package/tivana',
      },
    ],
  };
}
