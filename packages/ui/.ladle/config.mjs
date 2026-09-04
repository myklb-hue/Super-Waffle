/** @type {import('@ladle/react').UserConfig} */
export default {
  stories: 'src/**/*.stories.tsx',
  defaultStory: 'display--icons',
  addons: {
    a11y: { enabled: true },
    theme: { enabled: false },
    rtl: { enabled: false },
    mode: { enabled: false },
    width: { enabled: false },
    source: { enabled: true },
  },
};
