import React from 'react';

type PopupProps = {
  children: React.ReactNode;
};

export default function Popup({ children }: PopupProps) {
  return <div className="popup-shell">{children}</div>;
}
