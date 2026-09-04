import { Field, Icon, Label, Section, StatusDot, type IconName } from '@cyberloom/ui';
import { kind as lookupKind } from '@cyberloom/graph-core';
import { useDocument } from '../stores/document';
import { useRun } from '../stores/run';
import s from './RunPanel.module.css';

/**
 * What the inspector shows while a run is in flight (SPEC §8.5).
 *
 * Progress, live output, usage and the transport — the four things you want
 * while waiting, in the order you want them. It replaces the selection panel
 * for the duration rather than sitting beside it: the inspector has no
 * identity of its own and shows what matters now (SPEC §7.1), and while a
 * graph is running what matters is the run.
 */
export function RunPanel() {
  const phase = useRun((r) => r.phase);
  const order = useRun((r) => r.order);
  const blocks = useRun((r) => r.blocks);
  const live = useRun((r) => r.live);
  const usage = useRun((r) => r.usage);
  const results = useRun((r) => r.results);
  const failure = useRun((r) => r.failure);
  const halt = useRun((r) => r.halt);
  const graph = useDocument((d) => d.graph);

  const done = order.filter((id) => {
    const state = blocks[id]?.state;
    return state === 'done' || state === 'error';
  }).length;

  return (
    <>
      {failure && (
        <Section title="Did not start">
          <p className={s.failure}>{failure}</p>
        </Section>
      )}

      <Section
        title="Progress"
        right={
          order.length > 0 ? (
            <span className={s.progress}>
              {done} / {order.length}
            </span>
          ) : undefined
        }
      >
        {order.map((id) => {
          const block = graph.blocks.find((b) => b.id === id);
          const kind = block && lookupKind(block.kind);
          const run = blocks[id];
          return (
            <div key={id} className={s.step}>
              <Icon
                name={(kind?.icon ?? 'note') as IconName}
                size={12}
                color={`cat-${kind?.category ?? 'custom'}`}
                strokeWidth={1.7}
              />
              <span className={s.stepName}>{block?.title ?? kind?.title ?? id}</span>
              <span className={s.stepMs}>{run?.ms === undefined || run.ms === null ? '–' : ms(run.ms)}</span>
              <StatusDot
                state={
                  run?.state === 'running'
                    ? 'running'
                    : run?.state === 'done'
                      ? 'ok'
                      : run?.state === 'error'
                        ? 'error'
                        : run?.state === 'queued'
                          ? 'queued'
                          : 'idle'
                }
              />
            </div>
          );
        })}
      </Section>

      {live && (
        <Section title="Live output">
          <div className={s.live}>{live}</div>
        </Section>
      )}

      {results.length > 0 && (
        <Section title="Results">
          {results.map((result) => (
            <div key={result.port}>
              <Label>{result.port}</Label>
              <div className={s.result}>{text(result.value)}</div>
            </div>
          ))}
        </Section>
      )}

      <Section title="Usage">
        <Label>Tokens in</Label>
        <Field value={usage ? usage.tokensIn.toLocaleString() : '–'} mono />
        <Label>Tokens out</Label>
        <Field value={usage ? usage.tokensOut.toLocaleString() : '–'} mono />
        <Label>Cost</Label>
        {/* A local model is free, and saying so is more useful than a zero
            that looks like a number nobody filled in (SPEC §8.5). */}
        <Field
          value={usage ? (usage.local ? 'local · no charge' : 'remote · billed by the provider') : '–'}
        />
      </Section>

      {phase === 'running' && (
        <button type="button" className={s.stop} onClick={() => void halt()}>
          <Icon name="stop" size={11} strokeWidth={0} />
          Stop run
        </button>
      )}
    </>
  );
}

function ms(value: number): string {
  return value < 1000 ? `${value} ms` : `${(value / 1000).toFixed(1)} s`;
}

function text(value: { type: string; value?: unknown }): string {
  if (value.type === 'null') return '—';
  return typeof value.value === 'string' ? value.value : JSON.stringify(value.value, null, 2);
}
