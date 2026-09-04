import { useState } from 'react';
import { Button } from './components/Button';
import { Field } from './components/Field';
import { Segmented } from './components/Segmented';
import { Slider } from './components/Slider';
import { SwitchRow } from './components/SwitchRow';
import { TextBox } from './components/TextBox';
import { Toggle } from './components/Toggle';
import { Case, Row, Stack } from './story-helpers';

export const Fields = () => {
  const [name, setName] = useState('customer-triage.loom');
  return (
    <Stack>
      <Case name="editable">
        <Field value={name} onChange={setName} />
      </Case>
      <Case name="mono, with an icon and a suffix">
        <Field value="120" icon="clock" mono suffix="s" onChange={() => {}} />
      </Case>
      <Case name="select">
        <Field value="Local machine" icon="terminal" select onOpen={() => {}} />
      </Case>
      <Case name="static, muted">
        <Field value="What does this graph do?" muted />
      </Case>
      <Case name="disabled">
        <Field value="llama3.2:3b" select disabled />
      </Case>
      <Case name="loading">
        <Field value="" loading />
      </Case>
      <Case name="error">
        <Field
          value="/dev/ttyUSB9"
          mono
          error="No such device. Plug it in, or pick another port."
          onChange={() => {}}
        />
      </Case>
    </Stack>
  );
};

export const Sliders = () => {
  const [v, setV] = useState(0.62);
  return (
    <Stack>
      <Case name="default">
        <Slider label="Threshold" value={v} min={0} max={1} step={0.01} onChange={setV} />
      </Case>
      <Case name="with a unit and a type colour">
        <Slider
          label="Pan limit"
          value={40}
          min={-90}
          max={90}
          unit="deg"
          color="cat-actuators"
          onChange={() => {}}
        />
      </Case>
      <Case name="disabled">
        <Slider label="Temperature" value={0.7} min={0} max={2} step={0.1} disabled onChange={() => {}} />
      </Case>
    </Stack>
  );
};

export const Toggles = () => {
  const [on, setOn] = useState(true);
  return (
    <Stack>
      <Case name="bare">
        <Row>
          <Toggle on={on} label="Example" onChange={setOn} />
          <Toggle on={false} label="Off" onChange={() => {}} />
          <Toggle on color="ok" label="Success" onChange={() => {}} />
          <Toggle on disabled label="Disabled" onChange={() => {}} />
        </Row>
      </Case>
      <Case name="as a row, which is how it is normally used">
        <Stack gap={14}>
          <SwitchRow
            label="Keep aspect"
            hint="The rig stays square while you resize."
            on
            onChange={() => {}}
          />
          <SwitchRow
            label="Warn before running a shell command"
            hint="A prompt with a Continue. It never blocks the graph."
            on
            color="err"
            onChange={() => {}}
          />
          <SwitchRow
            label="Auto-affect from speech"
            hint="Off: an Affect block feeds express instead."
            on={false}
            onChange={() => {}}
          />
          <SwitchRow label="Record frames" hint="Unavailable while the graph is live." on={false} disabled onChange={() => {}} />
        </Stack>
      </Case>
    </Stack>
  );
};

export const Segments = () => {
  const [v, setV] = useState('Summary');
  return (
    <Stack>
      <Case name="default">
        <Segmented options={['Compact', 'Summary', 'Stage']} value={v} label="View" onChange={setV} />
      </Case>
      <Case name="two options, coloured">
        <Segmented options={['Inline', 'File']} value="Inline" color="cat-custom" label="Source" onChange={() => {}} />
      </Case>
      <Case name="disabled">
        <Segmented options={['Once', 'Live', 'Schedule']} value="Live" disabled label="Run mode" onChange={() => {}} />
      </Case>
    </Stack>
  );
};

export const Buttons = () => (
  <Stack>
    <Case name="variants">
      <Row>
        <Button label="Run" icon="play" variant="primary" />
        <Button label="Add rig" icon="plus" />
        <Button label="Delete" icon="stop" variant="danger" />
      </Row>
    </Case>
    <Case name="disabled">
      <Row>
        <Button label="Run" icon="play" variant="primary" disabled />
        <Button label="Add rig" disabled />
      </Row>
    </Case>
    <Case name="loading: the spinner replaces the icon, the width holds">
      <Row>
        <Button label="Run" icon="play" variant="primary" loading />
        <Button label="Reload" icon="loop" loading />
      </Row>
    </Case>
  </Stack>
);

export const TextBoxes = () => {
  const [v, setV] = useState('Answer the customer, then summarise what you did.');
  return (
    <Stack>
      <Case name="editable">
        <TextBox value={v} onChange={setV} />
      </Case>
      <Case name="empty, with a placeholder">
        <TextBox value="" placeholder="What does this graph do?" onChange={() => {}} />
      </Case>
      <Case name="mono">
        <TextBox value={'look: front door\nrecall: closed 2m ago\nact: pan -40'} mono minHeight={80} />
      </Case>
      <Case name="disabled">
        <TextBox value="Read-only while the graph is live." disabled />
      </Case>
    </Stack>
  );
};
