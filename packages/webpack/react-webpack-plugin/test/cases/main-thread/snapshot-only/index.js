/// <reference types="vitest/globals" />

import fs from 'node:fs/promises';

import './app.jsx';

// Both markers are assembled with `eval` so that the joined form never
// appears literally in this (bundled) test file.
const sideEffectMarker = eval(
  `['MODULE', 'SIDE', 'EFFECT', 'MARKER'].join('_')`,
);
const snapshotRegistration = eval(
  `['snapshot', 'Creator', 'Map['].join('')`,
);

const isMainThread = __filename.includes('main__main-thread');

it('should keep snapshot registrations in the main thread script', async () => {
  if (!isMainThread) {
    return;
  }
  const content = await fs.readFile(__filename, 'utf-8');
  expect(content).toContain(snapshotRegistration);
});

it('should not have business logic in the main thread script', async () => {
  if (!isMainThread) {
    return;
  }
  const content = await fs.readFile(__filename, 'utf-8');
  expect(content).not.toContain(sideEffectMarker);
});

it('should have business logic in the background script', async () => {
  if (isMainThread) {
    return;
  }
  const content = await fs.readFile(__filename, 'utf-8');
  expect(content).toContain(sideEffectMarker);
});
