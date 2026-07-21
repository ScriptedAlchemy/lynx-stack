import { useState } from '@lynx-js/react';

console.info('MODULE_SIDE_EFFECT_MARKER');

export function App() {
  const [count, setCount] = useState(0);
  return (
    <view>
      <text bindtap={() => setCount(count + 1)}>{count}</text>
    </view>
  );
}
