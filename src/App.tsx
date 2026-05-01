// ABOUTME: The main application component for EdMap Next-Gen.
// ABOUTME: Implements the VGA-style vertical sidebar and map viewport.

import React from 'react';
import './styles/turbo-vision.css';

const App: React.FC = () => {
  return (
    <div className="edmap-container">
      {/* Vertical Sidebar */}
      <aside className="edmap-sidebar">
        <div className="sidebar-header">
          <span className="header-title">EdMap</span>
          <span className="header-version">v1.40</span>
        </div>
        
        <div className="sidebar-menu">
          <div className="menu-item">Info</div>
          <div className="menu-item">File (map)</div>
          <div className="menu-item">WAD list</div>
          <div className="menu-item">Edit</div>
          <div className="menu-item">Map utilities</div>
          <div className="menu-item">Sectors</div>
          <div className="menu-item">Automatic</div>
          <div className="menu-item">Display</div>
          <div className="menu-item">Check</div>
        </div>

        <div className="sidebar-info-box">
          <div className="info-map-name">MAP 1</div>
          <div className="info-origin">original map</div>
          <div className="info-stats">214.82k free</div>
          <div className="info-help" style={{ marginTop: '4px', opacity: 0.8 }}>
            press F1<br />for help
          </div>
        </div>

        <div className="sidebar-tools">
          <div className="tool-section-header">Status</div>
          <div className="tool-row">
            <span className="tool-label">G:</span>
            <span className="tool-value">64</span>
            <span className="tool-label">S:</span>
            <span className="tool-value">8</span>
          </div>
          <div className="tool-row">
            <span className="tool-label">Z:</span>
            <span className="tool-value">1.00x</span>
          </div>
          <div className="tool-row">
            <span className="tool-value">-912, 1400</span>
          </div>

          <div className="tool-section-header" style={{ marginTop: '10px' }}>Selection</div>
          <div className="tool-row" style={{ color: 'var(--vga-cyan)' }}>
            <span>Vx</span> <span>Ld</span> <span>Se</span> <span>Th</span>
          </div>
          <div className="tool-row">
            <span className="tool-value" style={{ color: 'var(--vga-white)' }}>338/370</span>
          </div>
          
          <div className="selection-flags" style={{ marginTop: '4px', fontSize: '10px' }}>
            <div className="menu-item">• block all</div>
            <div className="menu-item">○ block enemy</div>
            <div className="menu-item">⁞ two-sided</div>
            <div className="menu-item">○ upper pegged</div>
            <div className="menu-item">○ lower pegged</div>
          </div>
        </div>
      </aside>

      {/* Map Viewport */}
      <main className="edmap-viewport">
        <div className="map-grid" />
        <svg width="100%" height="100%" viewBox="-2000 -2000 4000 4000">
          {/* Dummy Map Data for visual confirmation */}
          <path d="M -500 -500 L 500 -500 L 500 500 L -500 500 Z" className="line-linedef" fill="none" />
          <path d="M -800 -200 L -400 -200 L -400 200 L -800 200 Z" className="line-linedef" fill="none" />
          <circle cx="0" cy="0" r="10" fill="var(--vga-green)" />
          <text x="20" y="20" fill="var(--vga-gray)" fontSize="100">Sample View</text>
        </svg>
      </main>
    </div>
  );
};

export default App;
