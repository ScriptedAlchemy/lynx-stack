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
