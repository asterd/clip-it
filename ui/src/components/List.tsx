import React from 'react';
import { FixedSizeList as VirtualList, ListChildComponentProps } from 'react-window';
import {
  EyeIcon,
  FileTextIcon,
  FolderIcon,
  ImageIcon,
  PinIcon,
  StarIcon,
  TrashIcon
} from './icons';

type Item = {
  id: number;
  createdAt: number;
  kind: 'text' | 'image' | 'file' | string;
  text: string;
  previewText: string;
  imageWidth?: number | null;
  imageHeight?: number | null;
  favorite: boolean;
  pinned: boolean;
};

type Props = {
  items: Item[];
  selectedIndex: number;
  height: number;
  onHover: (index: number) => void;
  onSelect: (index: number) => void;
  onToggleFavorite: (item: Item) => void;
  onTogglePin: (item: Item) => void;
  onDelete: (item: Item) => void;
  onPreview: (item: Item) => void;
};

function previewFor(item: Item) {
  if (item.kind === 'image') return 'Image';
  if (item.kind === 'file') return item.text.split('\n')[0] || 'Path';
  return item.previewText || '(empty)';
}

function metaFor(item: Item) {
  if (item.kind === 'image' && item.imageWidth && item.imageHeight) {
    return `${item.imageWidth}x${item.imageHeight}`;
  }
  return new Date(item.createdAt).toLocaleTimeString();
}

function KindGlyph({ kind }: { kind: Item['kind'] }) {
  if (kind === 'image') return <ImageIcon className="kind-icon kind-image" />;
  if (kind === 'file') return <FolderIcon className="kind-icon kind-file" />;
  return <FileTextIcon className="kind-icon kind-text" />;
}

export default function List({
  items,
  selectedIndex,
  height,
  onHover,
  onSelect,
  onToggleFavorite,
  onTogglePin,
  onDelete,
  onPreview
}: Props) {
  const Row = ({ index, style }: ListChildComponentProps) => {
    const item = items[index];
    const selected = index === selectedIndex;

    return (
      <div
        style={style}
        className={`row ${selected ? 'selected' : ''}`}
        onMouseEnter={() => onHover(index)}
        onMouseDown={(e) => {
          e.preventDefault();
          onSelect(index);
        }}
      >
        <div className="row-head">
          <span className="kind" title={item.kind}>
            <KindGlyph kind={item.kind} />
          </span>
          <div className="row-main">
            <div className="row-title">{previewFor(item)}</div>
            <div className="row-meta">{metaFor(item)}</div>
          </div>
          <div className="row-actions">
            <button
              className="action"
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
                onPreview(item);
              }}
              title="Preview"
            >
              <EyeIcon className="action-icon" />
            </button>
            <button
              className="action"
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
                onToggleFavorite(item);
              }}
              title={item.favorite ? 'Unfavorite' : 'Favorite'}
            >
              <StarIcon className="action-icon" filled={item.favorite} />
            </button>
            <button
              className="action"
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
                onTogglePin(item);
              }}
              title={item.pinned ? 'Unpin' : 'Pin'}
            >
              <PinIcon className="action-icon" filled={item.pinned} />
            </button>
            <button
              className="action"
              onMouseDown={(e) => {
                e.stopPropagation();
                e.preventDefault();
                onDelete(item);
              }}
              title="Delete"
            >
              <TrashIcon className="action-icon" />
            </button>
          </div>
        </div>
      </div>
    );
  };

  return (
    <VirtualList
      className="list"
      itemCount={items.length}
      itemSize={54}
      width="100%"
      height={height}
      overscanCount={8}
    >
      {Row}
    </VirtualList>
  );
}
