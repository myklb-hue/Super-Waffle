import { useState } from 'react';
import { Button } from './components/Button';
import { Callout, DashedHint } from './components/Callout';
import { ConnectionRow } from './components/ConnectionRow';
import { EmptyState } from './components/EmptyState';
import { Field } from './components/Field';
import { Menu } from './components/Menu';
import { PanelHeader } from './components/PanelHeader';
import { Section } from './components/Section';
import { Tabs } from './components/Tabs';
import { Tooltip } from './components/Tooltip';
import { Case, Row, Stack } from './story-helpers';

export const Headers = () => {
  const [tab, setTab] = useState('Settings');
  return (
    <Stack gap={28}>
      <Case name="a block panel: the standard three tabs">
        <PanelHeader
          icon="llm"
          color="cat-models"
          title="Orchestrator"
          sub="models · llama3.2:3b"
          tabs={['Settings', 'Ports', 'Runs']}
          active={tab}
          onTab={setTab}
          onMenu={() => {}}
        />
      </Case>
      <Case name="a source: Events replaces Runs">
        <PanelHeader
          icon="eye"
          color="cat-senses"
          title="Webcam"
          sub="senses · /dev/video0"
          tabs={['Settings', 'Ports', 'Events']}
          active="Settings"
          onTab={() => {}}
        />
      </Case>
      <Case name="nothing selected: the panel falls back to the graph">
        <PanelHeader icon="mark" color="accent" title="Graph" sub="customer-triage.loom" />
      </Case>
    </Stack>
  );
};

export const TabStrips = () => {
  const [tab, setTab] = useState('Ports');
  return (
    <Stack>
      <Tabs tabs={['Settings', 'Ports', 'Runs']} active={tab} onChange={setTab} />
      <Tabs tabs={['Settings', 'Ports', 'Events']} active="Events" onChange={() => {}} />
      <Tabs tabs={['Console', 'Trace', 'Code']} active="Console" onChange={() => {}} />
    </Stack>
  );
};

export const Sections = () => (
  <Stack gap={0}>
    <Section title="Execution">
      <Field value="Local machine" icon="terminal" select onOpen={() => {}} />
      <Field value="4 parallel" select onOpen={() => {}} />
    </Section>
    <Section title="Privacy" tint="err">
      <Field value="Frames are never stored" muted />
    </Section>
    <Section title="Env and secrets" right={<Button label="Add" icon="plus" />}>
      <Field value="OPENAI_API_KEY" mono muted />
    </Section>
  </Stack>
);

export const Connections = () => (
  <Stack gap={6}>
    <ConnectionRow icon="toolbox" name="Toolbox" meta="tools · slot 1" kind="tools" state="ok" />
    <ConnectionRow icon="terminal" name="Terminal" meta="tool · handle" kind="tools" state="running" />
    <ConnectionRow icon="db" name="Long-term memory" meta="memory · sqlite" kind="memory" state="idle" />
    <ConnectionRow icon="eye" name="Webcam" meta="frames · 30 fps" kind="image" state="error" />
    <ConnectionRow icon="plug" name="Motors" meta="awaiting wire" kind="exec" state="pending" />
  </Stack>
);

export const Callouts = () => (
  <Stack>
    <Case name="a warning that never blocks">
      <Callout
        title="This will move the camera"
        body="motor.move(pan: -40). The prompt always has a Continue; nothing here stops the graph."
        color="warn"
      />
    </Case>
    <Case name="a privacy boundary">
      <Callout
        title="Faces are stored as embeddings"
        body="512 floats per person, never images. Delete a person and every sighting goes with them."
        color="err"
      />
    </Case>
    <Case name="a dashed hint: something absent">
      <DashedHint
        title="No secrets bound"
        body="Add one to expose it to Terminal and HTTP blocks."
        color="text-low"
      />
    </Case>
  </Stack>
);

export const EmptyStates = () => (
  <Stack>
    <Case name="an empty canvas">
      <EmptyState
        icon="plus"
        title="Drag a block from the library"
        hint="or press Cmd K to search all 44"
        actions={
          <>
            <Button label="Blank agent" />
            <Button label="Terminal assistant" />
          </>
        }
      />
    </Case>
    <Case name="an empty list">
      <EmptyState icon="clock" title="No runs yet" hint="Press R to run the graph" />
    </Case>
  </Stack>
);

export const Menus = () => (
  <Menu
    items={[
      { id: 'rename', label: 'Rename', icon: 'note', keys: 'F2' },
      { id: 'duplicate', label: 'Duplicate', icon: 'chunk', keys: 'Cmd D' },
      { id: 'library', label: 'Save to library', icon: 'folder' },
      { id: 'subgraph', label: 'Group into subgraph', icon: 'merge', keys: 'Cmd G', disabled: true },
      { id: 'delete', label: 'Delete', icon: 'stop', keys: 'Del', danger: true, separated: true },
    ]}
    onSelect={() => {}}
  />
);

export const Tooltips = () => (
  <Row gap={40}>
    <Tooltip content="tools · handle">
      <Button label="Hover me" />
    </Tooltip>
    <Tooltip content="data does not fit text. Insert a Convert block?">
      <Button label="Or focus me" />
    </Tooltip>
  </Row>
);
