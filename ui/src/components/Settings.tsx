import { useEffect, useRef, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { Command } from 'lucide-react';

type SettingsType = {
  hotkey: string;
  blur_close: boolean;
  polling_interval_ms: number;
  capture_enabled: boolean;
  max_items: number;
  window_opacity: number;
  colored_icons: boolean;
};

const MAX_ITEMS_OPTIONS = [10, 15, 25, 50, 100];

function formatHotkeyFromEvent(e: KeyboardEvent): string | null {
  const key = e.key.length === 1 ? e.key.toUpperCase() : e.key;
  const isModifierOnly = ['Shift', 'Control', 'Meta', 'Alt'].includes(key);
  if (isModifierOnly) return null;

  const parts: string[] = [];
  if (e.metaKey) parts.push('Cmd');
  if (e.ctrlKey) parts.push('Ctrl');
  if (e.altKey) parts.push('Alt');
  if (e.shiftKey) parts.push('Shift');

  const normalizedKey = key.length === 1 ? key : key.replace('Arrow', '');
  parts.push(normalizedKey);
  return parts.join('+');
}

export default function Settings() {
  const [loading, setLoading] = useState(true);
  const [hotkey, setHotkey] = useState('');
  const [blurClose, setBlurClose] = useState(true);
  const [maxItems, setMaxItems] = useState(15);
  const [windowOpacity, setWindowOpacity] = useState(78);
  const [coloredIcons, setColoredIcons] = useState(true);
  const [saving, setSaving] = useState(false);
  const [capturing, setCapturing] = useState(false);
  const [error, setError] = useState('');
  const captureRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    invoke<SettingsType>('get_settings')
      .then((s) => {
        setHotkey(s.hotkey);
        setBlurClose(s.blur_close);
        setMaxItems(s.max_items);
        setWindowOpacity(s.window_opacity ?? 78);
        setColoredIcons(s.colored_icons ?? true);
      })
      .finally(() => setLoading(false));
  }, []);

  useEffect(() => {
    const opacity = Math.max(35, Math.min(100, windowOpacity));
    document.documentElement.style.setProperty('--window-opacity', (opacity / 100).toFixed(2));
  }, [windowOpacity]);

  useEffect(() => {
    const onKeyDown = async (e: KeyboardEvent) => {
      if (capturing) {
        e.preventDefault();
        e.stopPropagation();
        const combo = formatHotkeyFromEvent(e);
        if (combo) {
          setHotkey(combo);
          setCapturing(false);
          captureRef.current?.blur();
        }
        return;
      }

      if (e.key === 'Escape') {
        e.preventDefault();
        await getCurrentWindow().hide();
      }
    };

    window.addEventListener('keydown', onKeyDown, true);
    return () => window.removeEventListener('keydown', onKeyDown, true);
  }, [capturing]);

  const save = async () => {
    setSaving(true);
    setError('');
    try {
      await invoke('set_setting', { key: 'hotkey', value: hotkey });
      await invoke('set_setting', { key: 'blur_close', value: blurClose });
      await invoke('set_setting', { key: 'max_items', value: maxItems });
      await invoke('set_setting', { key: 'window_opacity', value: windowOpacity });
      await invoke('set_setting', { key: 'colored_icons', value: coloredIcons });
      await getCurrentWindow().hide();
    } catch (e) {
      setError(String(e));
    } finally {
      setSaving(false);
    }
  };

  const close = async () => {
    await getCurrentWindow().hide();
  };

  const startDrag = async () => {
    await getCurrentWindow().startDragging();
  };

  if (loading) {
    return (
      <div className="glass-window flex h-screen w-screen items-center justify-center rounded-xl border border-slate-200/50 text-sm text-slate-500 shadow-2xl backdrop-blur-xl dark:border-slate-700/50 dark:text-slate-300">
        Loading settings...
      </div>
    );
  }

  return (
    <div className="glass-window flex h-screen w-screen flex-col overflow-hidden rounded-xl border border-slate-200/50 text-slate-800 shadow-2xl backdrop-blur-xl dark:border-slate-700/50 dark:text-slate-200">
      <div
        className="glass-header flex h-10 cursor-default select-none items-center gap-2 border-b border-slate-200/50 px-4 text-xs font-bold uppercase tracking-wider text-slate-400 dark:border-slate-700/50"
        data-tauri-drag-region
        onMouseDown={(e) => {
          if (e.button === 0) {
            startDrag();
          }
        }}
      >
        <Command size={14} />
        Clip It Settings
      </div>

      <div className="flex flex-1 flex-col gap-4 overflow-y-auto p-4">
        <label className="flex flex-col gap-2 text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">
          Popup hotkey
          <input
            ref={captureRef}
            className="h-11 rounded-xl border border-slate-200/70 bg-slate-100/80 px-3 text-sm text-slate-700 outline-none transition-all placeholder:text-slate-400 focus:border-primary/40 focus:ring-2 focus:ring-primary/40 dark:border-slate-700 dark:bg-slate-800/80 dark:text-slate-100"
            value={hotkey}
            onFocus={() => setCapturing(true)}
            onBlur={() => setCapturing(false)}
            onChange={(e) => setHotkey(e.target.value)}
            placeholder={navigator.userAgent.includes('Mac') ? 'Cmd+Shift+P' : 'Ctrl+Shift+P'}
          />
          <span className="text-[11px] normal-case text-slate-400">
            {capturing ? 'Press your key combination now...' : 'Focus field and press the combo.'}
          </span>
        </label>

        <label className="flex flex-col gap-2 text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">
          History size
          <div className="flex flex-wrap gap-2">
            {MAX_ITEMS_OPTIONS.map((opt) => (
              <button
                key={opt}
                className={[
                  'h-8 min-w-12 rounded-full border px-3 text-xs font-semibold transition-all',
                  maxItems === opt
                    ? 'border-primary/40 bg-primary text-white shadow-md shadow-primary/20'
                    : 'border-slate-200/80 bg-slate-100/80 text-slate-600 hover:bg-slate-200 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700'
                ].join(' ')}
                onClick={() => setMaxItems(opt)}
                type="button"
              >
                {opt}
              </button>
            ))}
          </div>
        </label>

        <label className="flex flex-col gap-2 text-xs font-medium uppercase tracking-wider text-slate-500 dark:text-slate-400">
          Trasparenza
          <div className="rounded-xl border border-slate-200/70 bg-slate-100/80 px-3 py-3 dark:border-slate-700 dark:bg-slate-800/80">
            <div className="mb-2 flex items-center justify-between text-[11px] normal-case text-slate-500 dark:text-slate-400">
              <span>Opacita finestra</span>
              <span className="rounded-md bg-slate-200/80 px-2 py-0.5 font-semibold text-slate-700 dark:bg-slate-700 dark:text-slate-200">
                {windowOpacity}%
              </span>
            </div>
            <input
              type="range"
              min={35}
              max={100}
              step={1}
              value={windowOpacity}
              onChange={(e) => setWindowOpacity(Number(e.target.value))}
              className="h-2 w-full cursor-pointer appearance-none rounded-lg bg-slate-300 accent-primary dark:bg-slate-600"
            />
          </div>
        </label>

        <label className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
          <input
            type="checkbox"
            checked={blurClose}
            onChange={(e) => setBlurClose(e.target.checked)}
            className="h-4 w-4 rounded border-slate-300 text-primary focus:ring-primary/50 dark:border-slate-600 dark:bg-slate-800"
          />
          Close popup on blur
        </label>

        <label className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
          <input
            type="checkbox"
            checked={coloredIcons}
            onChange={(e) => setColoredIcons(e.target.checked)}
            className="h-4 w-4 rounded border-slate-300 text-primary focus:ring-primary/50 dark:border-slate-600 dark:bg-slate-800"
          />
          Colored item icons
        </label>

        {error ? <div className="text-xs text-red-500">{error}</div> : null}
      </div>

      <div className="glass-header flex h-12 items-center justify-end gap-2 border-t border-slate-200/50 px-4 dark:border-slate-700/50">
        <button
          className="rounded-lg border border-slate-200/80 bg-slate-100/80 px-3 py-1.5 text-xs font-medium text-slate-600 transition-colors hover:bg-slate-200 dark:border-slate-700 dark:bg-slate-800 dark:text-slate-300 dark:hover:bg-slate-700"
          onClick={close}
        >
          Cancel
        </button>
        <button
          className="rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-white shadow-md shadow-primary/20 transition-opacity disabled:cursor-not-allowed disabled:opacity-50"
          onClick={save}
          disabled={saving || !hotkey.trim()}
        >
          {saving ? 'Saving...' : 'Save'}
        </button>
      </div>
    </div>
  );
}
