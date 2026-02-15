import { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import "./Layout.css";

interface LayoutProps {
  children: ReactNode;
  /** Current application phase for status bar */
  phase?: string;
  /** Status message to display */
  statusMessage?: string;
  /** Current version string */
  version?: string;
}

/**
 * Main application layout component.
 * Provides the overall structure: sidebar, main content area, and status bar.
 */
export function Layout({
  children,
  phase = "Ready",
  statusMessage,
  version,
}: LayoutProps) {
  return (
    <div className="layout">
      <Sidebar />
      <main className="layout-main">
        <div className="layout-content">{children}</div>
      </main>
      <StatusBar phase={phase} message={statusMessage} version={version} />
    </div>
  );
}
