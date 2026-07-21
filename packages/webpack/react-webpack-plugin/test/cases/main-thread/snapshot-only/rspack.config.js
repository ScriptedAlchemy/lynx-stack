import { createConfig } from '../../../create-react-config.js';

const defaultConfig = createConfig({
  experimental_mainThreadSnapshotOnly: true,
});

// `index.js` holds the assertions and must not be stripped from the
// main-thread bundle, so it is excluded from the react loaders.
const rules = defaultConfig.module.rules.map((rule) => {
  if (rule.issuerLayer) {
    return { ...rule, exclude: /snapshot-only[\\/]index\.js$/ };
  }
  return rule;
});

/** @type {import('@rspack/core').Configuration} */
export default {
  context: import.meta.dirname,
  ...defaultConfig,
  module: {
    rules,
  },
};
