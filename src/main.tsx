// ABOUTME: Entry point for the React frontend of EdMap Next-Gen.
// ABOUTME: Renders the App component into the DOM.

import React from 'react';
import ReactDOM from 'react-dom/client';
import App from './App';

ReactDOM.createRoot(document.getElementById('root') as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>
);
