import { clamp } from "./math.js";
const MAX = 100;
function normalize(value) {
    return clamp(value, 0, MAX);
}
__workletRuntimeLoaded && registerWorkletInternal("main-thread", "a123:test:1", function() {
    'main thread';
    return normalize(42);
});
