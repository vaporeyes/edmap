// ABOUTME: A React component that renders a window with the Borland Turbo Vision aesthetic.
// ABOUTME: Features a title bar, shadow, and classic color scheme.

import React from 'react';

interface TVWindowProps {
  title: string;
  children: React.ReactNode;
  width?: string;
  height?: string;
}

export const TVWindow: React.FC<TVWindowProps> = ({ title, children, width = '80%', height = '60%' }) => {
  return (
    <div className="tv-window-container" style={{ width, height }}>
      <div className="tv-window-shadow" />
      <div className="tv-window">
        <div className="tv-title-bar">
          <span className="tv-close">[■]</span>
          <span className="tv-title">{title}</span>
          <span className="tv-zoom">[↕]</span>
        </div>
        <div className="tv-content">
          {children}
        </div>
      </div>
    </div>
  );
};
