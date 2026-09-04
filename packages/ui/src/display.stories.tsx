import { Chip } from './components/Chip';
import { Icon } from './components/Icon';
import { KeyHint } from './components/KeyHint';
import { Label } from './components/Label';
import { Meter } from './components/Meter';
import { StatusDot } from './components/StatusDot';
import { TypeDot, TypeDots } from './components/TypeDot';
import { ICON_NAMES } from './components/icons';
import { PORT_TYPES, STATUS_STATES, CATEGORIES } from './types';
import { Case, Row, Stack } from './story-helpers';

export const Icons = () => (
  <Stack>
    <Case name="all 38 glyphs, 14px">
      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(8, 1fr)', gap: 14 }}>
        {ICON_NAMES.map((n) => (
          <span key={n} title={n} style={{ display: 'flex', justifyContent: 'center' }}>
            <Icon name={n} size={14} color="text-mid" />
          </span>
        ))}
      </div>
    </Case>
    <Case name="sizes">
      <Row>
        {([10, 12, 14, 18] as const).map((sz) => (
          <Icon key={sz} name="llm" size={sz} color="accent" />
        ))}
      </Row>
    </Case>
  </Stack>
);

export const StatusDots = () => (
  <Case name="every state">
    <Row gap={20}>
      {STATUS_STATES.map((st) => (
        <span key={st} style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
          <StatusDot state={st} />
          <span style={{ fontSize: 'var(--fs-sm)', color: 'var(--text-low)' }}>{st}</span>
        </span>
      ))}
    </Row>
  </Case>
);

export const Chips = () => (
  <Stack>
    <Case name="tinted">
      <Row gap={6}>
        <Chip label="python" color="cat-custom" />
        <Chip label="live" color="ok" dot />
        <Chip label="warn" color="warn" />
        <Chip label="paused" color="text-low" />
      </Row>
    </Case>
    <Case name="solid, and md">
      <Row gap={6}>
        <Chip label="run 18" color="accent" solid />
        <Chip label="local · ollama" color="text-mid" size="md" />
      </Row>
    </Case>
  </Stack>
);

export const PortTypes = () => (
  <Stack>
    <Case name="the ten types">
      <Stack gap={6}>
        {PORT_TYPES.map((t) => (
          <span key={t} style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <TypeDot kind={t} />
            <span style={{ fontFamily: 'var(--font-mono)', fontSize: 'var(--fs-sm)' }}>{t}</span>
          </span>
        ))}
      </Stack>
    </Case>
    <Case name="dimmed: cannot accept the drag in flight">
      <Row>
        {PORT_TYPES.slice(0, 5).map((t) => (
          <TypeDot key={t} kind={t} dim />
        ))}
      </Row>
    </Case>
    <Case name="summary on a library row">
      <Row gap={20}>
        <TypeDots kinds={['text', 'tools', 'memory']} />
        <TypeDots kinds={['image', 'data']} />
        <TypeDots kinds={['audio']} />
      </Row>
    </Case>
  </Stack>
);

export const Categories = () => (
  <Case name="one colour per category">
    <Stack gap={6}>
      {CATEGORIES.map((c) => (
        <Chip key={c} label={c} color={`cat-${c}`} dot />
      ))}
    </Stack>
  </Case>
);

export const Labels = () => (
  <Stack>
    <Label>Execution</Label>
    <Label>Env and secrets</Label>
  </Stack>
);

export const Meters = () => (
  <Stack>
    <Case name="microphone level">
      <Meter bars={[0.2, 0.5, 0.9, 0.6, 0.3, 0.75, 0.4, 0.15]} color="type-audio" label="Input level" />
    </Case>
    <Case name="token rate">
      <Meter bars={[0.4, 0.45, 0.6, 0.55, 0.7, 0.65]} color="ok" label="Tokens per second" />
    </Case>
    <Case name="silent">
      <Meter bars={[0, 0, 0, 0, 0, 0]} color="text-faint" label="Silent" />
    </Case>
  </Stack>
);

export const KeyHints = () => (
  <Row gap={16}>
    <KeyHint keys="Cmd K" />
    <KeyHint keys="Cmd E" />
    <KeyHint keys="Shift Drag" />
    <KeyHint keys="R" />
  </Row>
);
