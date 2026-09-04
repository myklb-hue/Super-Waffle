import { useState } from 'react';
import { CodeView, type Token } from './components/CodeView';
import { Grip } from './components/Grip';
import { ViewToggle } from './components/ViewToggle';
import type { View } from './types';
import { Case, Row, Stack } from './story-helpers';

export const ViewToggles = () => {
  const [a, setA] = useState<View>('summary');
  const [b, setB] = useState<View>('compact');
  return (
    <Stack>
      <Case name="a custom block: code is the third view">
        <Row>
          <ViewToggle active={a} third="code" onChange={setA} />
        </Row>
      </Case>
      <Case name="a block with a picture: stage is the third view">
        <Row>
          <ViewToggle active="stage" third="stage" onChange={() => {}} />
        </Row>
      </Case>
      <Case name="no third view: two positions, not a greyed-out third">
        <Row>
          <ViewToggle active={b} onChange={setB} />
        </Row>
      </Case>
    </Stack>
  );
};

export const Grips = () => {
  const [size, setSize] = useState({ w: 200, h: 140 });
  return (
    <Stack>
      <Case name="drag the corner; the block keeps its aspect if asked to">
        <div
          style={{
            position: 'relative',
            width: size.w,
            height: size.h,
            background: 'var(--block)',
            border: '1px solid var(--line)',
            borderRadius: 'var(--r-lg)',
            boxShadow: 'var(--shadow-block)',
            display: 'grid',
            placeItems: 'center',
            fontFamily: 'var(--font-mono)',
            fontSize: 'var(--fs-sm)',
            color: 'var(--text-low)',
          }}
        >
          {size.w} x {size.h}
          <Grip
            onResize={(dx, dy) =>
              setSize((s) => ({
                w: Math.max(168, s.w + dx),
                h: Math.max(80, s.h + dy),
              }))
            }
          />
        </div>
      </Case>
      <Case name="disabled">
        <div
          style={{
            position: 'relative',
            width: 160,
            height: 70,
            background: 'var(--block)',
            border: '1px solid var(--line)',
            borderRadius: 'var(--r-lg)',
          }}
        >
          <Grip disabled onResize={() => {}} />
        </div>
      </Case>
    </Stack>
  );
};

const SOURCE: Token[][] = [
  [
    { text: 'def ', kind: 'keyword' },
    { text: 'door_check', kind: 'func' },
    { text: '(' },
    { text: 'frame' },
    { text: ': ' },
    { text: 'Image', kind: 'type' },
    { text: ', ' },
    { text: 'threshold' },
    { text: ': ' },
    { text: 'float', kind: 'type' },
    { text: ' = ' },
    { text: '0.6', kind: 'number' },
    { text: ') -> ' },
    { text: 'Data', kind: 'type' },
    { text: ':' },
  ],
  [{ text: '    "Is the front door open?"', kind: 'string' }],
  [{ text: '    # the signature is the interface', kind: 'comment' }],
  [
    { text: '    score = ' },
    { text: 'detect', kind: 'func' },
    { text: '(frame)' },
  ],
  [
    { text: '    ' },
    { text: 'return', kind: 'keyword' },
    { text: ' {' },
    { text: '"open"', kind: 'string' },
    { text: ': score > threshold}' },
  ],
];

export const CodeViews = () => (
  <Stack>
    <Case name="source, with the signature marked">
      <CodeView lines={SOURCE} marks={[1]} />
    </Case>
    <Case name="an error on line 4">
      <CodeView lines={SOURCE} errorLine={4} />
    </Case>
    <Case name="loading">
      <CodeView lines={[]} loading height={120} />
    </Case>
    <Case name="empty">
      <CodeView lines={[]} height={90} />
    </Case>
  </Stack>
);
