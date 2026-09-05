import { useEffect, useState } from 'react';
import type { WorkspaceInfo, WorkspaceSettings } from '@cyberloom/graph-core';
import { Button, Callout, Chip, Field, Label, Section, StatusDot, SwitchRow } from '@cyberloom/ui';
import { pullModel, workspaceSettings, configureWorkspace } from '../stores/rpc';
import { useRun } from '../stores/run';
import s from './Settings.module.css';

/**
 * The workspace settings screen (`/w/settings`).
 *
 * Two halves, and the split is the point. What is *installed* is detected every
 * time this opens and is not editable: someone who has just installed ffmpeg in
 * another window should see that by coming back here rather than by restarting
 * the application. What was *chosen* is four overrides, and they live in a file
 * in the folder, so a workspace carries its intentions with it.
 *
 * It is also the first-run experience (slice 12): a fresh machine opens this and
 * is told, in one screen, what is missing and what to do about each thing.
 */
export function Settings({ onClose }: { onClose: () => void }) {
  const [info, setInfo] = useState<WorkspaceInfo | null>(null);
  const [draft, setDraft] = useState<WorkspaceSettings | null>(null);
  const [saving, setSaving] = useState<string | null>(null);

  useEffect(() => {
    void workspaceSettings().then((got) => {
      setInfo(got);
      setDraft(got.settings);
    });
  }, []);

  if (!info || !draft) {
    return (
      <div className={s.screen}>
        <div className={s.waiting}>Looking at this machine…</div>
      </div>
    );
  }

  const change = <K extends keyof WorkspaceSettings>(key: K, value: WorkspaceSettings[K]) =>
    setDraft({ ...draft, [key]: value });
  // Look again, keeping what is being typed: a pull that finished should turn
  // the Ollama row's model count over without losing an unsaved field.
  const refresh = () => workspaceSettings().then((got) => setInfo(got));

  const found = [info.probe.python, info.probe.ffmpeg, info.probe.ollama, info.probe.models];
  const missing = found.filter((f) => !f.ok).length;

  return (
    <div className={s.screen} data-testid="settings">
      <header className={s.head}>
        <div>
          <div className={s.title}>Workspace</div>
          <div className={s.path}>{info.root}</div>
        </div>
        <Button label="Back to the canvas" onClick={onClose} />
      </header>

      <div className={s.columns}>
        <div>
          <Section
            title="On this machine"
            right={
              <Chip
                label={missing === 0 ? 'all here' : `${missing} missing`}
                color={missing === 0 ? 'ok' : 'warn'}
                dot
              />
            }
          >
            {found.map((thing) => (
              <div key={thing.name} className={s.found} data-testid={`probe-${thing.name}`}>
                <StatusDot state={thing.ok ? 'ok' : 'error'} />
                <div className={s.foundText}>
                  <div className={s.foundName}>{thing.name}</div>
                  <div className={s.foundDetail}>{thing.detail}</div>
                  {thing.fix && <div className={s.foundFix}>{thing.fix}</div>}
                  {thing.name === 'Ollama' && thing.ok && (
                    <Pull model={draft.model ?? 'llama3.2:3b'} onDone={() => void refresh()} />
                  )}
                </div>
              </div>
            ))}
            <div className={s.note}>
              Detected now, every time this screen opens. Nothing here is written
              down: an engine that remembered where Python was in March is an
              engine that is wrong in April.
            </div>
          </Section>
        </div>

        <div>
          <Section title="What this workspace chooses">
            <Label>Python</Label>
            <Field
              value={draft.python ?? ''}
              placeholder="python3"
              mono
              onChange={(v) => change('python', v || null)}
            />
            <Label>Models</Label>
            <Field
              value={draft.models ?? ''}
              placeholder="models"
              mono
              onChange={(v) => change('models', v || null)}
            />
            <Label>Ollama</Label>
            <Field
              value={draft.ollama ?? ''}
              placeholder="http://127.0.0.1:11434"
              mono
              onChange={(v) => change('ollama', v || null)}
            />
            <Label>Default model</Label>
            <Field
              value={draft.model ?? ''}
              placeholder="llama3.2:3b"
              mono
              onChange={(v) => change('model', v || null)}
            />
            <SwitchRow
              label="New graphs are local only"
              hint="On. A graph with this on sends nothing to a service that is not on this machine."
              on={draft.localOnlyDefault}
              color="err"
              onChange={(on) => change('localOnlyDefault', on)}
            />
            <div className={s.actions}>
              <Button
                label={saving === 'saving' ? 'Saving…' : 'Save'}
                variant="primary"
                onClick={() => {
                  setSaving('saving');
                  void configureWorkspace(draft)
                    .then((got) => {
                      setInfo(got);
                      setDraft(got.settings);
                      setSaving('saved');
                    })
                    .catch((e: unknown) => setSaving(e instanceof Error ? e.message : String(e)));
                }}
              />
              {saving && saving !== 'saving' && <span className={s.saved}>{saving}</span>}
            </div>
          </Section>

          {missing > 0 && (
            <Callout
              title="Offline is a supported state"
              body="A graph that only uses blocks whose tools are here will run. What is missing stops those blocks and says so, on the block, when you run it."
              color="text-low"
              dashed
            />
          )}
        </div>
      </div>
    </div>
  );
}

/**
 * Pull a model into Ollama from here, and watch it arrive (SPEC §15.13:
 * "model downloads explicit and resumable"). Ollama's own pull resumes what
 * it already has, so a second click after a dropped connection continues.
 */
function Pull({ model, onDone }: { model: string; onDone: () => void }) {
  const progress = useRun((r) => r.progress[model]);
  const [asked, setAsked] = useState<string | null>(null);
  const busy = !!progress && !progress.done;
  const fraction = progress && progress.total > 0 ? progress.completed / progress.total : null;

  useEffect(() => {
    if (progress?.done && !progress.error) onDone();
    // The probe is what changes; onDone is stable enough for this.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [progress?.done]);

  return (
    <div className={s.pull} data-testid={`pull-${model}`}>
      <Button
        label={busy ? `Pulling ${model}…` : `Pull ${model}`}
        icon="llm"
        loading={busy}
        disabled={busy}
        onClick={() => {
          setAsked(null);
          pullModel(model).catch((e: unknown) => setAsked(e instanceof Error ? e.message : String(e)));
        }}
      />
      {progress && (
        <div className={s.pullState}>
          <div className={s.bar}>
            <div
              className={s.barFill}
              style={{ width: `${Math.round((fraction ?? (progress.done ? 1 : 0)) * 100)}%` }}
            />
          </div>
          <div className={s.pullLine}>
            {progress.error
              ? progress.error
              : progress.done
                ? `${model} is here`
                : `${progress.status}${fraction !== null ? ` · ${Math.round(fraction * 100)}% of ${(progress.total / 1e9).toFixed(1)} GB` : ''}`}
          </div>
        </div>
      )}
      {asked && <div className={s.pullLine}>{asked}</div>}
    </div>
  );
}
