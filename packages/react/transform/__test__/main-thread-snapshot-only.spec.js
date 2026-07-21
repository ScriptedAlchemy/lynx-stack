// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

import { describe, expect, it } from 'vitest';

import { transformReactLynx } from '../main.js';

const options = {
  mode: 'test',
  pluginName: '',
  filename: 'test.jsx',
  sourcemap: false,
  cssScope: false,
  jsx: true,
  directiveDCE: { target: 'LEPUS' },
  defineDCE: {
    define: {
      __LEPUS__: 'true',
      __MAIN_THREAD__: 'true',
      __JS__: 'false',
      __BACKGROUND__: 'false',
    },
  },
  shake: true,
  compat: false,
  worklet: {
    target: 'LEPUS',
    filename: 'test.jsx',
    runtimePkg: '@lynx-js/react/internal',
  },
  refresh: false,
  mainThreadSnapshotOnly: true,
  snapshot: {
    preserveJsx: false,
    runtimePkg: '@lynx-js/react/internal',
    jsxImportSource: '@lynx-js/react/lepus',
    target: 'LEPUS',
    filename: 'test',
  },
};

describe('mainThreadSnapshotOnly', () => {
  it('should keep only snapshot registrations on the main thread', async () => {
    const result = await transformReactLynx(
      `
import { useState, useEffect } from "@lynx-js/react";
import { track } from "my-monitor";

track("module-side-effect");

export function App() {
  const [count, setCount] = useState(0);
  useEffect(() => {
    track("effect");
  }, []);
  return (
    <view>
      <text bindtap={() => setCount(count + 1)}>{count}</text>
    </view>
  );
}
`,
      options,
    );

    expect(result.code).toContain('snapshotCreatorMap');
    expect(result.code).not.toContain('useState');
    expect(result.code).not.toContain('track');
    expect(result.code).not.toContain('function App');
    expect(result.code).toMatchSnapshot();
  });

  it('should keep the module graph through side-effect imports', async () => {
    const result = await transformReactLynx(
      `
import Counter from "./comp-lib/index.jsx";
import { root } from "@lynx-js/react";

root.render(__MAIN_THREAD__ ? null : <Counter />);
`,
      options,
    );

    expect(result.code).toContain('import "./comp-lib/index.jsx"');
    expect(result.code).not.toContain('root.render');
  });

  it('should keep full output when disabled', async () => {
    const result = await transformReactLynx(
      `
import { useState } from "@lynx-js/react";

export function App() {
  const [count, setCount] = useState(0);
  return <view><text>{count}</text></view>;
}
`,
      { ...options, mainThreadSnapshotOnly: false },
    );

    expect(result.code).toContain('function App');
  });
});
