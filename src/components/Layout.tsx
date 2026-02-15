import { ReactNode } from "react";
import { Sidebar } from "./Sidebar";
import { StatusBar } from "./StatusBar";
import "./Layout.css";

/** Link configuration for sidebar navigation. */
interface SidebarLink {
  label: string;
  href?: string;
  onClick?: () => void;
  icon?: string;
}

interface LayoutProps {
  children: ReactNode;
  /** Current application phase for status bar */
  phase?: string;
  /** Status message to display */
  statusMessage?: string;
  /** Current version string */
  version?: string;
  /** Custom sidebar navigation links */
  sidebarLinks?: SidebarLink[];
  /** Callback when settings is clicked */
  onSettingsClick?: () => void;
  /** Callback when home is clicked */
  onHomeClick?: () => void;
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
  sidebarLinks,
  onSettingsClick,
  onHomeClick,
}: LayoutProps) {
  // Build default links with navigation callbacks
  const defaultLinks: SidebarLink[] = [
    { label: "Home", icon: "\uD83C\uDFE0", onClick: onHomeClick },
    { label: "Settings", icon: "\u2699\uFE0F", onClick: onSettingsClick },
    { label: "Help", icon: "\u2753" },
  ];

  const links = sidebarLinks || defaultLinks;

  return (
    <div className="layout">
      <Sidebar links={links} />
      <main className="layout-main">
        <div className="layout-content">{children}</div>
      </main>
      <StatusBar phase={phase} message={statusMessage} version={version} />
    </div>
  );
}
