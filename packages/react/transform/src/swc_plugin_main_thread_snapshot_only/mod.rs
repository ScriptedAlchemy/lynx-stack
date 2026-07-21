// Copyright 2026 The Lynx Authors. All rights reserved.
// Licensed under the Apache License Version 2.0 that can be found in the
// LICENSE file in the root directory of this source tree.

//! Strips a main-thread (LEPUS) module down to the statements the main thread
//! must keep for background-driven rendering: `snapshotCreatorMap`
//! registrations and worklet registrations, plus every top-level declaration
//! they transitively reference. All other top-level statements — component
//! functions, hooks, module side effects — are removed.
//!
//! Imports are never dropped: unreferenced ones are turned into side-effect
//! imports (and re-exports into side-effect imports of their source) so the
//! module graph stays intact and every module still contributes its snapshot
//! registrations to the main-thread bundle.

use rustc_hash::{FxHashMap, FxHashSet};
use swc_core::ecma::{
  ast::*,
  visit::{noop_visit_mut_type, noop_visit_type, Visit, VisitMut, VisitWith},
};

pub struct MainThreadSnapshotOnlyVisitor;

/// Detects the statements emitted by the snapshot and worklet plugins:
/// `<runtime>.snapshotCreatorMap[id] = ...` and
/// `<loaded> && registerWorkletInternal(...)`.
struct RootDetector {
  found: bool,
}

impl Visit for RootDetector {
  noop_visit_type!();

  fn visit_member_expr(&mut self, n: &MemberExpr) {
    if let MemberProp::Ident(prop) = &n.prop {
      if prop.sym == "snapshotCreatorMap" {
        self.found = true;
        return;
      }
    }
    n.visit_children_with(self);
  }

  fn visit_call_expr(&mut self, n: &CallExpr) {
    if let Callee::Expr(callee) = &n.callee {
      if let Expr::Ident(ident) = &**callee {
        if ident.sym == "registerWorkletInternal" {
          self.found = true;
          return;
        }
      }
    }
    n.visit_children_with(self);
  }
}

struct IdentCollector {
  ids: FxHashSet<Id>,
}

impl Visit for IdentCollector {
  noop_visit_type!();

  fn visit_ident(&mut self, n: &Ident) {
    self.ids.insert(n.to_id());
  }
}

fn is_root(item: &ModuleItem) -> bool {
  if let ModuleItem::Stmt(Stmt::Expr(_)) = item {
    let mut detector = RootDetector { found: false };
    item.visit_with(&mut detector);
    detector.found
  } else {
    false
  }
}

fn collect_pat_ids(pat: &Pat, ids: &mut Vec<Id>) {
  match pat {
    Pat::Ident(ident) => ids.push(ident.to_id()),
    Pat::Array(arr) => {
      for elem in arr.elems.iter().flatten() {
        collect_pat_ids(elem, ids);
      }
    }
    Pat::Object(obj) => {
      for prop in &obj.props {
        match prop {
          ObjectPatProp::KeyValue(kv) => collect_pat_ids(&kv.value, ids),
          ObjectPatProp::Assign(assign) => ids.push(assign.key.to_id()),
          ObjectPatProp::Rest(rest) => collect_pat_ids(&rest.arg, ids),
        }
      }
    }
    Pat::Rest(rest) => collect_pat_ids(&rest.arg, ids),
    Pat::Assign(assign) => collect_pat_ids(&assign.left, ids),
    Pat::Expr(_) | Pat::Invalid(_) => {}
  }
}

fn collect_decl_ids(decl: &Decl, ids: &mut Vec<Id>) {
  match decl {
    Decl::Var(var) => {
      for declarator in &var.decls {
        collect_pat_ids(&declarator.name, ids);
      }
    }
    Decl::Fn(f) => ids.push(f.ident.to_id()),
    Decl::Class(c) => ids.push(c.ident.to_id()),
    _ => {}
  }
}

fn declared_ids(item: &ModuleItem) -> Vec<Id> {
  let mut ids = vec![];
  match item {
    ModuleItem::Stmt(Stmt::Decl(decl)) => collect_decl_ids(decl, &mut ids),
    ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export)) => {
      collect_decl_ids(&export.decl, &mut ids)
    }
    ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
      for specifier in &import.specifiers {
        match specifier {
          ImportSpecifier::Named(named) => ids.push(named.local.to_id()),
          ImportSpecifier::Default(default) => ids.push(default.local.to_id()),
          ImportSpecifier::Namespace(ns) => ids.push(ns.local.to_id()),
        }
      }
    }
    ModuleItem::ModuleDecl(ModuleDecl::ExportDefaultDecl(export)) => match &export.decl {
      DefaultDecl::Fn(f) => {
        if let Some(ident) = &f.ident {
          ids.push(ident.to_id());
        }
      }
      DefaultDecl::Class(c) => {
        if let Some(ident) = &c.ident {
          ids.push(ident.to_id());
        }
      }
      DefaultDecl::TsInterfaceDecl(_) => {}
    },
    _ => {}
  }
  ids
}

fn side_effect_import(src: Box<Str>, with: Option<Box<ObjectLit>>) -> ModuleItem {
  ModuleItem::ModuleDecl(ModuleDecl::Import(ImportDecl {
    span: Default::default(),
    specifiers: vec![],
    src,
    type_only: false,
    with,
    phase: Default::default(),
  }))
}

impl VisitMut for MainThreadSnapshotOnlyVisitor {
  noop_visit_mut_type!();

  fn visit_mut_module(&mut self, module: &mut Module) {
    let len = module.body.len();
    let mut keep = vec![false; len];
    let mut declared: FxHashMap<Id, usize> = FxHashMap::default();
    let mut worklist: Vec<usize> = vec![];

    for (index, item) in module.body.iter().enumerate() {
      if is_root(item) {
        keep[index] = true;
        worklist.push(index);
      }
      for id in declared_ids(item) {
        declared.insert(id, index);
      }
    }

    while let Some(index) = worklist.pop() {
      let mut collector = IdentCollector {
        ids: FxHashSet::default(),
      };
      module.body[index].visit_with(&mut collector);
      for id in collector.ids {
        if let Some(&target) = declared.get(&id) {
          if !keep[target] {
            keep[target] = true;
            worklist.push(target);
          }
        }
      }
    }

    let mut new_body = Vec::with_capacity(len);
    for (index, item) in module.body.drain(..).enumerate() {
      match item {
        ModuleItem::ModuleDecl(ModuleDecl::Import(mut import)) => {
          if !keep[index] {
            import.specifiers.clear();
          }
          new_body.push(ModuleItem::ModuleDecl(ModuleDecl::Import(import)));
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => {
          if let Some(src) = export.src {
            new_body.push(side_effect_import(src, export.with));
          }
        }
        ModuleItem::ModuleDecl(ModuleDecl::ExportAll(export)) => {
          new_body.push(side_effect_import(export.src, export.with));
        }
        item if keep[index] => new_body.push(item),
        _ => {}
      }
    }
    module.body = new_body;
  }
}

#[cfg(test)]
mod tests {
  use swc_core::{
    ecma::parser::{Syntax, TsSyntax},
    ecma::transforms::testing::test,
    ecma::visit::visit_mut_pass,
  };

  use crate::swc_plugin_main_thread_snapshot_only::MainThreadSnapshotOnlyVisitor;

  fn syntax() -> Syntax {
    Syntax::Typescript(TsSyntax {
      tsx: true,
      ..Default::default()
    })
  }

  test!(
    module,
    syntax(),
    |_| visit_mut_pass(MainThreadSnapshotOnlyVisitor),
    should_keep_snapshot_registrations_and_drop_business_logic,
    r#"
    import * as ReactLynx from "@lynx-js/react/internal";
    import { useState } from "@lynx-js/react";
    import { track } from "my-monitor";
    track("module-side-effect");
    const __snapshot_da39a_test_1 = "__snapshot_da39a_test_1";
    ReactLynx.snapshotCreatorMap[__snapshot_da39a_test_1] = (__snapshot_da39a_test_1)=>ReactLynx.createSnapshot(__snapshot_da39a_test_1, function() {
        const pageId = ReactLynx.__pageId;
        const el = __CreateView(pageId);
        return [el];
    }, null, null, undefined, globDynamicComponentEntry, null, true);
    export function App() {
        const [count, setCount] = useState(0);
        return <__snapshot_da39a_test_1/>;
    }
    "#
  );

  test!(
    module,
    syntax(),
    |_| visit_mut_pass(MainThreadSnapshotOnlyVisitor),
    should_keep_worklet_registrations_with_dependencies,
    r#"
    import { loadWorkletRuntime as __loadWorkletRuntime } from "@lynx-js/react";
    var loadWorkletRuntime = __loadWorkletRuntime;
    let onScroll = {
        _wkltId: "a123:test:1"
    };
    const __workletRuntimeLoaded = loadWorkletRuntime(typeof globDynamicComponentEntry === 'undefined' ? undefined : globDynamicComponentEntry);
    __workletRuntimeLoaded && registerWorkletInternal("main-thread", "a123:test:1", function(enable) {
        const onScroll = lynxWorkletImpl._workletMap["a123:test:1"].bind(this);
        'main thread';
        console.log(enable);
    });
    export function App() {
        return null;
    }
    "#
  );

  test!(
    module,
    syntax(),
    |_| visit_mut_pass(MainThreadSnapshotOnlyVisitor),
    should_preserve_module_graph_with_side_effect_imports,
    r#"
    import Counter from "./comp-lib/index.jsx";
    import "./index.css";
    import { root } from "@lynx-js/react";
    export * from "./re-exported.js";
    export { named } from "./named.js";
    root.render(<Counter/>);
    "#
  );

  test!(
    module,
    syntax(),
    |_| visit_mut_pass(MainThreadSnapshotOnlyVisitor),
    should_keep_transitive_dependencies_of_worklet,
    r#"
    import { clamp } from "./math.js";
    const MAX = 100;
    function normalize(value) {
        return clamp(value, 0, MAX);
    }
    const unusedHelper = () => 1;
    __workletRuntimeLoaded && registerWorkletInternal("main-thread", "a123:test:1", function() {
        'main thread';
        return normalize(42);
    });
    "#
  );

  test!(
    module,
    syntax(),
    |_| visit_mut_pass(MainThreadSnapshotOnlyVisitor),
    should_drop_default_exports_and_bare_statements,
    r#"
    import { setup } from "./setup.js";
    setup();
    export default function Home() {
        return null;
    }
    export const config = { a: 1 };
    if (globalThis.__DEV__) {
        console.log("dev");
    }
    "#
  );
}
