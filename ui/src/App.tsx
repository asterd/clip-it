import type { MouseEvent } from 'react';
import { useCallback, useEffect, useRef, useState } from 'react';
import { motion, AnimatePresence } from 'framer-motion';
import {
  Search,
  Settings as SettingsIcon,
  X,
  Copy,
  Eye,
  Star,
  Pin,
  Trash2,
  Image as ImageIcon,
  FileText,
  Type,
  Command
} from 'lucide-react';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import Settings from './components/Settings';

type ClipboardItem = {
  id: number;
  kind: 'text' | 'image' | 'file' | string;
  text?: string;
  previewText?: string;
  imageWidth?: number | null;
  imageHeight?: number | null;
  createdAt: string | number;
  favorite: boolean;
  pinned: boolean;
};

type SearchResponse = {
  total: number;
  items: ClipboardItem[];
};

type UiSettings = {
  window_opacity: number;
  blur_close: boolean;
  colored_icons: boolean;
};

type ItemPreview = {
  kind: 'text' | 'image' | 'file' | string;
  text: string;
  imageRgba?: number[];
  imageWidth?: number;
  imageHeight?: number;
};

type FilterType = 'all' | 'favorites' | 'pinned';

const isSettingsView = new URLSearchParams(window.location.search).get('view') === 'settings';
const SEARCH_DEBOUNCE_MS = 80;
const isMacOS = /Mac|iPhone|iPad|iPod/i.test(navigator.platform || navigator.userAgent);
const shortcutModifierLabel = isMacOS ? 'CMD' : 'CTRL';

function formatCreatedAt(value: string | number): string {
  const date = typeof value === 'number' ? new Date(value) : new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function imageDataUrlFromRgba(rgba: number[], width?: number, height?: number): string {
  if (!width || !height || rgba.length === 0) return '';
  const canvas = document.createElement('canvas');
  canvas.width = width;
  canvas.height = height;
  const ctx = canvas.getContext('2d');
  if (!ctx) return '';
  const imageData = new ImageData(new Uint8ClampedArray(rgba), width, height);
  ctx.putImageData(imageData, 0, 0);
  return canvas.toDataURL('image/png');
}

function rowShortcutLabel(index: number): string {
  const keyLabel = index === 9 ? '0' : String(index + 1);
  return `${shortcutModifierLabel}+${keyLabel}`;
}

function shortcutIndexFromEvent(event: KeyboardEvent): number | null {
  if (!(event.metaKey || event.ctrlKey) || event.altKey || event.shiftKey) return null;

  const key = event.key;
  if (key >= '1' && key <= '9') return Number(key) - 1;
  if (key === '0') return 9;

  switch (event.code) {
    case 'Digit1':
    case 'Numpad1':
      return 0;
    case 'Digit2':
    case 'Numpad2':
      return 1;
    case 'Digit3':
    case 'Numpad3':
      return 2;
    case 'Digit4':
    case 'Numpad4':
      return 3;
    case 'Digit5':
    case 'Numpad5':
      return 4;
    case 'Digit6':
    case 'Numpad6':
      return 5;
    case 'Digit7':
    case 'Numpad7':
      return 6;
    case 'Digit8':
    case 'Numpad8':
      return 7;
    case 'Digit9':
    case 'Numpad9':
      return 8;
    case 'Digit0':
    case 'Numpad0':
      return 9;
    default:
      return null;
  }
}

export default function App() {
  const [query, setQuery] = useState('');
  const [items, setItems] = useState<ClipboardItem[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);
  const [filter, setFilter] = useState<FilterType>('all');
  const [toast, setToast] = useState<string | null>(null);
  const [previewOpen, setPreviewOpen] = useState(false);
  const [preview, setPreview] = useState<ItemPreview | null>(null);
  const [previewItemId, setPreviewItemId] = useState<number | null>(null);
  const [coloredIcons, setColoredIcons] = useState(true);

  const searchInputRef = useRef<HTMLInputElement>(null);
  const blurCloseRef = useRef(true);

  const loadItems = useCallback(async (nextQuery: string, nextFilter: FilterType) => {
    try {
      const res = await invoke<SearchResponse>('search_items', {
        query: nextQuery,
        limit: 200,
        offset: 0,
        filter: nextFilter
      });

      const nextItems = res?.items ?? [];
      setItems(nextItems);
      setSelectedIndex((index) => {
        if (nextItems.length === 0) return 0;
        return Math.min(index, nextItems.length - 1);
      });
    } catch (error) {
      console.error('Failed to load items', error);
    }
  }, []);

  const showToast = (message: string) => {
    setToast(message);
    setTimeout(() => setToast(null), 1800);
  };

  const hideWindow = async () => {
    await getCurrentWindow().hide();
  };

  const handleSelect = async (item: ClipboardItem) => {
    await invoke('set_clipboard_item', { itemId: item.id });
    showToast('Copiato!');
    setTimeout(() => {
      hideWindow();
    }, 280);
  };

  const handleDelete = async (event: MouseEvent, item: ClipboardItem) => {
    event.stopPropagation();
    await invoke('delete_item', { itemId: item.id });
    await loadItems(query, filter);
  };

  const handleTogglePin = async (event: MouseEvent, item: ClipboardItem) => {
    event.stopPropagation();
    await invoke('pin_item', { itemId: item.id, pinned: !item.pinned });
    await loadItems(query, filter);
  };

  const handleToggleFavorite = async (event: MouseEvent, item: ClipboardItem) => {
    event.stopPropagation();
    await invoke('favorite_item', { itemId: item.id, favorite: !item.favorite });
    await loadItems(query, filter);
  };

  const openSettings = async () => {
    await invoke('open_settings_window');
    await hideWindow();
  };

  const clearAllItems = async () => {
    const confirmed = window.confirm('Vuoi cancellare tutti gli elementi della cronologia?');
    if (!confirmed) return;
    await invoke('clear_all_history');
    await loadItems(query, filter);
    showToast('Cronologia svuotata');
  };

  const openPreview = async (event: MouseEvent, item: ClipboardItem) => {
    event.stopPropagation();
    const data = await invoke<ItemPreview>('get_item_preview', { itemId: item.id });
    setPreview(data);
    setPreviewItemId(item.id);
    setPreviewOpen(true);
  };

  const openPathFromPreview = async () => {
    if (!previewItemId) return;
    await invoke('open_item_path', { itemId: previewItemId });
  };

  const applyUiSettings = async () => {
    try {
      const settings = await invoke<UiSettings>('get_settings');
      const opacity = Math.max(35, Math.min(100, settings.window_opacity ?? 78));
      document.documentElement.style.setProperty('--window-opacity', (opacity / 100).toFixed(2));
      blurCloseRef.current = settings.blur_close ?? true;
      setColoredIcons(settings.colored_icons ?? true);
    } catch {
      document.documentElement.style.setProperty('--window-opacity', '0.78');
      blurCloseRef.current = true;
      setColoredIcons(true);
    }
  };

  useEffect(() => {
    document.documentElement.dataset.view = isSettingsView ? 'settings' : 'main';
    return () => {
      delete document.documentElement.dataset.view;
    };
  }, []);

  useEffect(() => {
    if (isSettingsView) return;
    applyUiSettings();

    const handle = setTimeout(() => {
      loadItems(query, filter);
    }, SEARCH_DEBOUNCE_MS);

    return () => clearTimeout(handle);
  }, [loadItems, query, filter]);

  useEffect(() => {
    if (isSettingsView) return;

    const onPopupOpened = listen('popup:opened', async () => {
      setQuery('');
      setFilter('all');
      setPreviewOpen(false);
      setPreview(null);
      setPreviewItemId(null);
      await applyUiSettings();
      await loadItems('', 'all');
      setSelectedIndex(0);
      setTimeout(() => searchInputRef.current?.focus(), 0);
    });

    const onItemAdded = listen('clipboard:item_added', async () => {
      if (query.trim() === '') {
        await loadItems('', filter);
      }
    });

    const onBlur = async () => {
      if (blurCloseRef.current) {
        await hideWindow();
      }
    };

    window.addEventListener('blur', onBlur);

    return () => {
      onPopupOpened.then((unlisten) => unlisten());
      onItemAdded.then((unlisten) => unlisten());
      window.removeEventListener('blur', onBlur);
    };
  }, [filter, loadItems, query]);

  useEffect(() => {
    if (isSettingsView) return;

    const handleKeyDown = async (event: KeyboardEvent) => {
      if (!previewOpen) {
        const shortcutIndex = shortcutIndexFromEvent(event);
        if (shortcutIndex !== null) {
          event.preventDefault();
          const shortcutItem = items[shortcutIndex];
          if (shortcutItem) {
            await handleSelect(shortcutItem);
          }
          return;
        }
      }

      if (previewOpen && event.key === 'Escape') {
        event.preventDefault();
        setPreviewOpen(false);
        return;
      }

      if (event.key === 'ArrowDown') {
        event.preventDefault();
        setSelectedIndex((index) => Math.min(index + 1, Math.max(items.length - 1, 0)));
        return;
      }

      if (event.key === 'ArrowUp') {
        event.preventDefault();
        setSelectedIndex((index) => Math.max(index - 1, 0));
        return;
      }

      if (event.key === 'Enter') {
        event.preventDefault();
        if (items[selectedIndex]) {
          await handleSelect(items[selectedIndex]);
        }
        return;
      }

      if (event.key === 'Escape') {
        event.preventDefault();
        if (query) {
          setQuery('');
        } else {
          await hideWindow();
        }
      }
    };

    window.addEventListener('keydown', handleKeyDown);
    return () => window.removeEventListener('keydown', handleKeyDown);
  }, [items, previewOpen, query, selectedIndex]);

  if (isSettingsView) {
    return <Settings />;
  }

  return (
    <div className="glass-window relative h-screen w-screen overflow-hidden rounded-xl border border-slate-200/50 text-slate-800 shadow-2xl backdrop-blur-xl dark:border-slate-700/50 dark:text-slate-200">
      <div
        data-tauri-drag-region
        className="glass-header flex h-9 cursor-default select-none items-center justify-between border-b border-slate-200/50 px-3 dark:border-slate-700/50"
      >
        <div className="flex items-center gap-2 text-xs font-bold uppercase tracking-wider text-slate-400">
          <Command size={13} />
          Clip It
        </div>
        <div className="flex items-center gap-1">
          <button
            onClick={clearAllItems}
            className="rounded-md p-1 text-slate-400 transition-colors hover:bg-red-100 hover:text-red-600 dark:hover:bg-red-900/30 dark:hover:text-red-300"
            title="Clear all history"
          >
            <Trash2 size={13} />
          </button>
          <button
            onClick={openSettings}
            className="rounded-md p-1 text-slate-400 transition-colors hover:bg-slate-200 hover:text-slate-600 dark:hover:bg-slate-700 dark:hover:text-slate-200"
            title="Settings"
          >
            <SettingsIcon size={13} />
          </button>
        </div>
      </div>

      <div className="p-2.5 pb-1.5">
        <div className="group relative">
          <Search
            className="absolute left-2.5 top-1/2 -translate-y-1/2 text-slate-400 transition-colors group-focus-within:text-primary"
            size={15}
          />
          <input
            ref={searchInputRef}
            autoFocus
            value={query}
            onChange={(event) => setQuery(event.target.value)}
            className="w-full rounded-lg bg-slate-100/80 py-2 pl-8 pr-7 text-xs outline-none transition-all placeholder:text-slate-400 focus:ring-2 focus:ring-primary/50 dark:bg-slate-800/80"
            placeholder="Cerca appunti..."
          />
          {query ? (
            <button
              onClick={() => setQuery('')}
              className="absolute right-2 top-1/2 -translate-y-1/2 text-slate-400 hover:text-slate-600"
            >
              <X size={12} />
            </button>
          ) : null}
        </div>

        <div className="scrollbar-hide mt-2 flex gap-1.5 overflow-x-auto pb-0.5">
          {(['all', 'favorites', 'pinned'] as FilterType[]).map((value) => (
            <button
              key={value}
              onClick={() => setFilter(value)}
              className={[
                'h-6 rounded-full px-2.5 text-[11px] font-medium capitalize transition-all',
                filter === value
                  ? 'bg-primary text-white shadow-md shadow-primary/20'
                  : 'bg-slate-100 text-slate-500 hover:bg-slate-200 dark:bg-slate-800 dark:hover:bg-slate-700'
              ].join(' ')}
            >
              {value}
            </button>
          ))}
        </div>
      </div>

      <div className="flex-1 space-y-0.5 overflow-y-auto px-1.5 pb-8">
        {items.length === 0 ? (
          <div className="flex h-full flex-col items-center justify-center text-sm text-slate-400">
            <Copy size={32} className="mb-2 opacity-20" />
            <p>Nessun elemento trovato</p>
          </div>
        ) : (
          items.map((item, index) => (
            <motion.div
              layoutId={`item-${item.id}`}
              key={item.id}
              initial={{ opacity: 0, y: 8 }}
              animate={{ opacity: 1, y: 0 }}
              onClick={() => handleSelect(item)}
              onMouseEnter={() => setSelectedIndex(index)}
              className={[
                'group flex cursor-pointer items-center gap-2 rounded-md border border-transparent p-1.5 transition-all duration-150',
                index === selectedIndex
                  ? 'border-primary/20 bg-primary/10 shadow-sm'
                  : 'hover:bg-slate-100 dark:hover:bg-slate-800/50'
              ].join(' ')}
            >
              <div
                className={[
                  'shrink-0 rounded-md p-1.5',
                  index === selectedIndex
                    ? 'bg-primary text-white'
                    : coloredIcons
                    ? item.kind === 'image'
                      ? 'bg-amber-100 text-amber-600 dark:bg-amber-900/30 dark:text-amber-300'
                      : item.kind === 'file'
                      ? 'bg-emerald-100 text-emerald-600 dark:bg-emerald-900/30 dark:text-emerald-300'
                      : 'bg-sky-100 text-sky-600 dark:bg-sky-900/30 dark:text-sky-300'
                    : 'bg-slate-200 text-slate-500 dark:bg-slate-700'
                ].join(' ')}
              >
                {item.kind === 'image' ? (
                  <ImageIcon size={14} />
                ) : item.kind === 'file' ? (
                  <FileText size={14} />
                ) : (
                  <Type size={14} />
                )}
              </div>

              <div className="min-w-0 flex-1">
                <p
                  className={[
                    'truncate text-xs font-medium leading-tight',
                    index === selectedIndex ? 'text-primary' : 'text-slate-700 dark:text-slate-200'
                  ].join(' ')}
                >
                  {item.previewText || item.text || 'Contenuto vuoto'}
                </p>
                <div className="mt-0.5 flex items-center gap-1.5">
                  <span className="text-[9px] text-slate-400">{formatCreatedAt(item.createdAt)}</span>
                  {item.imageWidth && item.imageHeight ? (
                    <span className="rounded bg-slate-200 px-1 text-[9px] text-slate-500 dark:bg-slate-700">
                      {item.imageWidth}x{item.imageHeight}
                    </span>
                  ) : null}
                </div>
              </div>

              <div className="flex items-center">
                {index < 10 ? (
                  <span
                    className={[
                      'mr-1 flex items-center rounded bg-slate-200 px-1.5 text-[9px] font-semibold text-slate-500 transition-opacity dark:bg-slate-700 dark:text-slate-300',
                      index === selectedIndex ? 'opacity-0' : 'opacity-100 group-hover:opacity-0'
                    ].join(' ')}
                  >
                    {rowShortcutLabel(index)}
                  </span>
                ) : null}
                <div
                  className={[
                    'flex gap-1 transition-opacity',
                    index === selectedIndex ? 'opacity-100' : 'opacity-0 group-hover:opacity-100'
                  ].join(' ')}
                >
                <button
                  onClick={(event) => openPreview(event, item)}
                  className="rounded p-1 text-slate-400 hover:bg-slate-200 hover:text-slate-600 dark:hover:bg-slate-600 dark:hover:text-slate-200"
                  title="Preview"
                >
                  <Eye size={12} />
                </button>
                <button
                  onClick={(event) => handleTogglePin(event, item)}
                  className={[
                    'rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-600',
                    item.pinned ? 'text-orange-500' : 'text-slate-400'
                  ].join(' ')}
                  title="Pin"
                >
                  <Pin size={12} className={item.pinned ? 'fill-current' : ''} />
                </button>
                <button
                  onClick={(event) => handleToggleFavorite(event, item)}
                  className={[
                    'rounded p-1 hover:bg-slate-200 dark:hover:bg-slate-600',
                    item.favorite ? 'text-yellow-500' : 'text-slate-400'
                  ].join(' ')}
                  title="Favorite"
                >
                  <Star size={12} className={item.favorite ? 'fill-current' : ''} />
                </button>
                <button
                  onClick={(event) => handleDelete(event, item)}
                  className="rounded p-1 text-slate-400 hover:bg-red-100 hover:text-red-500 dark:hover:bg-red-900/30"
                  title="Delete"
                >
                  <Trash2 size={12} />
                </button>
                </div>
              </div>
            </motion.div>
          ))
        )}
      </div>

      <div className="glass-header absolute bottom-0 left-0 right-0 flex h-7 items-center justify-between border-t border-slate-200/50 px-3 text-[9px] text-slate-400 dark:border-slate-700/50">
        <span>{items.length} elementi</span>
        <div className="flex items-center gap-2">
          <span className="flex items-center gap-1">
            <span className="rounded bg-slate-200 px-1 dark:bg-slate-700">↵</span> seleziona
          </span>
          <span className="flex items-center gap-1">
            <span className="rounded bg-slate-200 px-1 dark:bg-slate-700">⌘/ctrl+1..0</span> quick copy
          </span>
          <span className="flex items-center gap-1">
            <span className="rounded bg-slate-200 px-1 dark:bg-slate-700">esc</span> chiudi
          </span>
        </div>
      </div>

      {previewOpen && preview ? (
        <div
          className="absolute inset-0 z-40 flex items-center justify-center bg-slate-950/35 p-3"
          onMouseDown={() => setPreviewOpen(false)}
        >
          <div
            className="max-h-[82%] w-[min(560px,95%)] overflow-hidden rounded-xl border border-slate-200/70 bg-white/90 shadow-2xl backdrop-blur-xl dark:border-slate-700 dark:bg-slate-900/90"
            onMouseDown={(event) => event.stopPropagation()}
          >
            <div className="flex h-10 items-center justify-between border-b border-slate-200/50 bg-slate-50/50 px-3 text-xs font-semibold text-slate-500 dark:border-slate-700/50 dark:bg-slate-800/50 dark:text-slate-300">
              <span>Preview</span>
              <button
                className="rounded p-1 text-slate-400 hover:bg-slate-200 hover:text-slate-600 dark:hover:bg-slate-700 dark:hover:text-slate-100"
                onClick={() => setPreviewOpen(false)}
              >
                <X size={14} />
              </button>
            </div>

            {preview.kind === 'image' ? (
              <div className="flex max-h-[64vh] items-center justify-center overflow-auto p-3">
                <img
                  className="max-h-[58vh] max-w-full rounded-lg border border-slate-200/70 dark:border-slate-700"
                  src={imageDataUrlFromRgba(
                    preview.imageRgba ?? [],
                    preview.imageWidth,
                    preview.imageHeight
                  )}
                  alt="Clipboard preview"
                />
              </div>
            ) : (
              <div className="flex max-h-[64vh] flex-col gap-3 overflow-auto p-3">
                <pre className="whitespace-pre-wrap break-words text-xs leading-relaxed text-slate-700 dark:text-slate-200">
                  {preview.text}
                </pre>
                {preview.kind === 'file' ? (
                  <button
                    className="w-fit rounded-lg bg-primary px-3 py-1.5 text-xs font-semibold text-white shadow-md shadow-primary/20"
                    onClick={openPathFromPreview}
                  >
                    Open in Finder/Explorer
                  </button>
                ) : null}
              </div>
            )}
          </div>
        </div>
      ) : null}

      <AnimatePresence>
        {toast ? (
          <motion.div
            initial={{ opacity: 0, y: 20 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: 20 }}
            className="absolute bottom-12 left-1/2 z-50 -translate-x-1/2 rounded-full bg-slate-800 px-4 py-2 text-xs font-medium text-white shadow-lg"
          >
            {toast}
          </motion.div>
        ) : null}
      </AnimatePresence>
    </div>
  );
}
