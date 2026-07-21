---
"@lynx-js/react": minor
"@lynx-js/react-webpack-plugin": minor
"@lynx-js/react-rsbuild-plugin": minor
---

Add `experimental_mainThreadSnapshotOnly` to strip the main-thread bundle down to snapshot and worklet registrations.

With this option enabled, business logic (component functions, hooks and module side effects) no longer runs on the main thread. The main-thread bundle keeps only what background-driven rendering needs: `snapshotCreatorMap` registrations, worklet registrations and their dependencies, with the module graph preserved through side-effect imports. The first frame is empty and the UI is rendered by the background thread through hydration.

Modules inside `node_modules` and the ReactLynx runtime are not stripped, so dependencies keep working and the main-thread boot stays intact.
